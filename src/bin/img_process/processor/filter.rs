use crate::util::{BuilderExtManualExt as _, Image, Result};
use failure::ensure;
use gtk::{prelude::*, Builder};
use ndarray::prelude::*;
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
                    .round() as usize;
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
        type Ty = (FilterType, usize, f32, f32, f32);
        let (filter_ty, neighbor, gauss_sigma, bila_sigma_d, bila_sigma_r): Ty =
            *args.downcast_ref().unwrap();
        let src = src.expect_normal()?;
        let (h, w, _) = src.dim();
        ensure!(neighbor % 2 == 1, "Kernel size should be odd number");
        ensure!(
            neighbor <= h && neighbor <= w,
            "Kernel should not be larger than image",
        );

        let dest = match filter_ty {
            FilterType::Box => box_filter(src, neighbor),
            FilterType::Gaussian => gauss_filter(src, neighbor, gauss_sigma),
            FilterType::Wiener => wiener_filter(src, neighbor),
            FilterType::Bilateral => bilateral_filter(src, neighbor, bila_sigma_d, bila_sigma_r),
        };
        Ok(Image::Normal(dest))
    }
}

/// Kernel:
/// K(x, y) = A * 1
fn box_filter(src: Array3<f32>, kernel_size: usize) -> Array3<f32> {
    // Normalize factor.
    let k = 1.0 / kernel_size.pow(2) as f32;
    let kernel = Array::from_elem((kernel_size, kernel_size), k);
    linear_filter(src, kernel)
}

/// Kernel:
/// G(x, y) = 1/(2πσ^2) * e^((-x^2-y^2)/σ^2)
fn gauss_filter(src: Array3<f32>, kernel_size: usize, sigma: f32) -> Array3<f32> {
    let mid = (kernel_size / 2) as f32;
    let mut kernel = Array::from_shape_fn((kernel_size, kernel_size), |(x, y)| {
        let (x, y) = (x as f32, y as f32);
        ((-(x - mid).powi(2) - (y - mid).powi(2)) / sigma.powi(2)).exp()
    });
    // Normalize.
    kernel /= kernel.sum();
    linear_filter(src, kernel)
}

fn linear_filter(src: Array3<f32>, kernel: Array2<f32>) -> Array3<f32> {
    let (ksize, ksize_) = kernel.dim();
    assert_eq!(ksize, ksize_);
    assert!(ksize > 0 && ksize % 2 == 1);

    let (h, w, ncol) = src.dim();
    assert_eq!(ncol, 3);
    let (h2, w2) = (h - ksize, w - ksize);
    let mut dest = Array::zeros((h2, w2, 3));
    for ((x, y, c), v) in dest.indexed_iter_mut() {
        let mut sum = 0.0;
        for i in 0..ksize {
            for j in 0..ksize {
                sum += src[[x + i, y + j, c]] * kernel[[i, j]];
            }
        }
        *v = sum;
    }

    dest
}

fn wiener_filter(src: Array3<f32>, neighbor: usize) -> Array3<f32> {
    // let (h, w, ncol) = src.dim();
    // assert_eq!(ncol, 3);
    // let mut mean = Array::zeros((h, w, 3));
    todo!()
}

fn bilateral_filter(src: Array3<f32>, neighbor: usize, sigma_d: f32, sigma_r: f32) -> Array3<f32> {
    todo!()
}
