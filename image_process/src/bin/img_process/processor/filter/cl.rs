use crate::util::{OptionExt as _, Result};
use ndarray::prelude::*;

const OPENCL_KERNEL_SRC: &str = include_str!("./kernel.cl");

pub fn linear_filter(src: Array3<f32>, kernel: Array2<f32>) -> Result<Array3<f32>> {
    use ocl::{
        enums::{ImageChannelDataType, ImageChannelOrder, MemObjectType},
        Context, Device, Image, Kernel, Program, Queue,
    };

    let (h, w, _) = src.dim();
    let (ksize, ksize_) = kernel.dim();
    assert_eq!(ksize, ksize_);

    let kernel_buf = kernel.into_raw_vec();

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

    let kernel_image = Image::<f32>::builder()
        // `read_imagef` will get (RGBA)(I, I, I, 1.0).
        .channel_order(ImageChannelOrder::Intensity)
        .channel_data_type(ImageChannelDataType::Float)
        .image_type(MemObjectType::Image2d)
        .dims((ksize, ksize))
        .flags(ocl::flags::MEM_READ_ONLY | ocl::flags::MEM_HOST_WRITE_ONLY)
        .copy_host_slice(&kernel_buf)
        .queue(queue.clone())
        .build()?;

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
        .dims((w, h))
        .flags(ocl::flags::MEM_WRITE_ONLY | ocl::flags::MEM_HOST_READ_ONLY)
        .queue(queue.clone())
        .build()?;

    let kernel = Kernel::builder()
        .name("linear_transform")
        .program(&program)
        .queue(queue.clone())
        .global_work_size((w, h))
        .arg(&kernel_image)
        .arg(&src_image)
        .arg(&dest_image)
        .build()?;

    unsafe { kernel.enq()? };

    // RGBA output
    let mut buf = vec![0.0f32; h * w * 4];
    dest_image.read(&mut buf[..]).enq()?;
    // Convert back to RGB
    let dest_rgba = Array::from_shape_vec((h, w, 4), buf).unwrap();
    Ok(dest_rgba.slice(s![.., .., ..3]).to_owned())
}
