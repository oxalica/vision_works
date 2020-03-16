use crate::util::{BuilderExtManualExt as _, Image, Result};
use failure::ensure;
use gtk::{prelude::*, Builder};
use std::any::Any;

pub struct Filter;

#[derive(Clone, Copy)]
enum FilterType {
    Box,
    Gaussian,
    Wiener,
    Bilateral,
}

impl super::ImageProcessor for Filter {
    fn register_handler(
        &self,
        builder: &Builder,
        handler_name: &str,
        run: Box<dyn Fn(Box<dyn Any + Send>) + 'static>,
    ) -> Option<Box<dyn Fn() + 'static>> {
        let on_filter = |filter_ty: FilterType| {
            let builder = builder.clone();
            Box::new(move || {
                let neighbor = builder
                    .object::<gtk::Scale>("scl_filter_neighbor")
                    .get_value()
                    .round() as i32;
                let gauss_sigma = builder
                    .object::<gtk::Scale>("scl_filter_gauss_sigma")
                    .get_value() as f32;
                let bila_sigma_d = builder
                    .object::<gtk::Scale>("scl_filter_bilateral_sigma_d")
                    .get_value() as f32;
                let bila_sigma_r = builder
                    .object::<gtk::Scale>("scl_filter_bilateral_sigma_r")
                    .get_value() as f32;
                run(Box::new((
                    filter_ty,
                    neighbor,
                    gauss_sigma,
                    bila_sigma_d,
                    bila_sigma_r,
                )));
            })
        };

        match handler_name {
            "on_filter_run_box" => Some(on_filter(FilterType::Box)),
            "on_filter_run_gauss" => Some(on_filter(FilterType::Gaussian)),
            "on_filter_run_wiener" => Some(on_filter(FilterType::Wiener)),
            "on_filter_run_bilateral" => Some(on_filter(FilterType::Bilateral)),
            _ => None,
        }
    }

    fn run(&self, args: Box<dyn Any + Send>, src: Image) -> Result<Image> {
        type Ty = (FilterType, i32, f32, f32, f32);
        let (filter_ty, neighbor, gauss_sigma, bila_sigma_d, bila_sigma_r): Ty =
            *args.downcast_ref().unwrap();
        // match filter_ty {
        //     FilterType::Box => box_filter(src, neighbor),
        //     FilterType::Gaussian => gauss_filter(src, neighbor, gauss_sigma),
        //     FilterType::Wiener => wiener_filter(src, neighbor),
        //     FilterType::Bilateral => bilateral_filter(src, neighbor, bila_sigma_d, bila_sigma_r),
        // }
        todo!()
    }
}
/*

/// Kernel:
/// K(x, y) = A * 1
fn box_filter(src: Mat, kernel_size: i32) -> Result<Mat> {
    // Normalize factor.
    let k = 1.0 / (kernel_size * kernel_size) as f64;
    let kernel = Mat::new_rows_cols_with_default(kernel_size, kernel_size, CV_32F, Scalar::all(k))?;
    linear_filter(src, kernel)
}

/// Kernel:
/// G(x, y) = 1/(2πσ^2) * e^((-x^2-y^2)/σ^2)
fn gauss_filter(src: Mat, kernel_size: i32, sigma: f32) -> Result<Mat> {
    let mut kernel =
        Mat::new_rows_cols_with_default(kernel_size, kernel_size, CV_32F, Scalar::all(0.0))?;
    let mid = kernel_size / 2;
    let mut sum = 0.0;
    for x in 0..kernel_size {
        for y in 0..kernel_size {
            let v = ((-(x - mid).pow(2) - (y - mid).pow(2)) as f32 / sigma.powi(2)).exp();
            sum += v;
            *kernel.at_2d_mut::<f32>(x, y).unwrap() = v;
        }
    }
    // Normalize.
    for x in 0..kernel_size {
        for y in 0..kernel_size {
            *kernel.at_2d_mut::<f32>(x, y).unwrap() /= sum;
        }
    }
    linear_filter(src, kernel)
}

fn linear_filter(src: Mat, kernel: Mat) -> Result<Mat> {
    let ksize = kernel.rows();
    ensure!(
        kernel.rows() == kernel.cols(),
        "Only square kernel is supported",
    );
    ensure!(
        ksize > 0 && ksize % 2 == 1,
        "Kernel width should be an positive odd number",
    );

    let (h, w) = (src.rows(), src.cols());
    ensure!(h >= ksize && w >= ksize, "Image is too small");

    let (h2, w2) = (h - ksize, w - ksize);
    let mut dest = Mat::new_rows_cols_with_default(h2, w2, CV_8UC3, Scalar::all(0.0))?;
    for x in 0..h2 {
        for y in 0..w2 {
            let (mut r, mut g, mut b) = (0.0, 0.0, 0.0);
            for i in 0..ksize {
                for j in 0..ksize {
                    let s = src.at_2d::<Vec3b>(x + i, y + j).unwrap().0;
                    let k = *kernel.at_2d::<f32>(i, j).unwrap();
                    r += s[0] as f32 * k;
                    g += s[1] as f32 * k;
                    b += s[2] as f32 * k;
                }
            }
            let r = r.round().max(0.0).min(255.0) as u8;
            let g = g.round().max(0.0).min(255.0) as u8;
            let b = b.round().max(0.0).min(255.0) as u8;
            dest.at_2d_mut::<Vec3b>(x, y).unwrap().0 = [r, g, b];
        }
    }

    Ok(dest)
}

fn wiener_filter(src: Mat, neighbor: i32) -> Result<Mat> {
    todo!()
}

fn bilateral_filter(src: Mat, neighbor: i32, sigma_d: f32, sigma_r: f32) -> Result<Mat> {
    todo!()
}
*/
