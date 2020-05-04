use super::{get_rotation_mat, get_scale_mat, get_size_after_affine_trans, get_translate_mat};
use crate::util::{OptionExt as _, Result};
use ndarray::prelude::*;

const OPENCL_KERNEL_SRC: &str = include_str!("./kernel.cl");

pub fn affine_trans(src: Array3<f32>, scale: f32, rotate: f32) -> Result<Array3<f32>> {
    use ocl::{
        enums::{ImageChannelDataType, ImageChannelOrder, MemObjectType},
        prm, Context, Device, Image, Kernel, Program, Queue,
    };

    let (h, w, _) = src.dim();
    let (h2, w2) = get_size_after_affine_trans(h, w, scale, rotate);

    // Most OpenCL implementations support only f32-RGBA image,
    // so we need to expand it first.
    let src_rgba_buf = Array::from_shape_fn((h, w, 4), |(x, y, col)| {
        if col != 3 {
            src[[x, y, col]]
        } else {
            // Alpha
            1.0
        }
    })
    .into_raw_vec();

    // Inverse matrix. So we can get source points for each destination points.
    let inv_trans_mat = get_translate_mat(h as f32 / 2.0, w as f32 / 2.0)
        .dot(&get_rotation_mat(-rotate))
        .dot(&get_scale_mat(1.0 / scale))
        .dot(&get_translate_mat(-(h2 as f32 / 2.0), -(w2 as f32 / 2.0)));
    let mat_flatten = prm::Float8::from([
        inv_trans_mat[[0, 0]],
        inv_trans_mat[[0, 1]],
        inv_trans_mat[[0, 2]],
        inv_trans_mat[[1, 0]],
        inv_trans_mat[[1, 1]],
        inv_trans_mat[[1, 2]],
        // Unused
        0.0,
        0.0,
    ]);

    // Setup OpenCL

    let context = Context::builder()
        .devices(Device::specifier().first())
        .build()?;
    let device = *context.devices().get(0).context("No OpenCL device")?;

    let queue = Queue::new(&context, device, None)?;
    let program = Program::builder()
        .src(OPENCL_KERNEL_SRC)
        .devices(device)
        .build(&context)?;

    let src_image = Image::<f32>::builder()
        .channel_order(ImageChannelOrder::Rgba)
        .channel_data_type(ImageChannelDataType::Float)
        .image_type(MemObjectType::Image2d)
        .dims((w, h))
        .flags(ocl::flags::MEM_READ_ONLY | ocl::flags::MEM_HOST_WRITE_ONLY)
        .copy_host_slice(&src_rgba_buf)
        .queue(queue.clone())
        .build()?;

    let dest_image = Image::<f32>::builder()
        .channel_order(ImageChannelOrder::Rgba)
        .channel_data_type(ImageChannelDataType::Float)
        .image_type(MemObjectType::Image2d)
        .dims(&(w2, h2))
        .flags(ocl::flags::MEM_WRITE_ONLY | ocl::flags::MEM_HOST_READ_ONLY)
        .queue(queue.clone())
        .build()?;

    let kernel = Kernel::builder()
        .name("affine_transform")
        .program(&program)
        .queue(queue.clone())
        .global_work_size((w2, h2))
        .arg(&mat_flatten)
        .arg(&src_image)
        .arg(&dest_image)
        .build()?;

    unsafe { kernel.enq()? };

    // RGBA output
    let mut buf = vec![0.0f32; h2 * w2 * 4];
    dest_image.read(&mut buf[..]).enq()?;
    // Convert back to RGB
    let dest_rgba = Array::from_shape_vec((h2, w2, 4), buf).unwrap();
    Ok(dest_rgba.slice(s![.., .., ..3]).to_owned())
}
