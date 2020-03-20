use crate::util::{BuilderExtManualExt as _, Image, Result};
use failure::ensure;
use gtk::{prelude::*, Builder};
use ndarray::{prelude::*, Zip};
use std::any::Any;

mod cl;

pub struct Filter;

#[derive(Clone, Copy)]
enum FilterType {
    Box,
    Gaussian,
    GaussianCL,
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
            "on_filter_run_gauss_ocl" => Some(on_filter(FilterType::GaussianCL)),
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
            FilterType::Box => linear_filter(src, box_filter_kernel(neighbor)),
            FilterType::Gaussian => linear_filter(src, gauss_filter_kernel(neighbor, gauss_sigma)),
            FilterType::GaussianCL => {
                cl::linear_filter(src, gauss_filter_kernel(neighbor, gauss_sigma))?
            }
            FilterType::Wiener => wiener_filter(src, neighbor),
            FilterType::Bilateral => bilateral_filter(src, neighbor, bila_sigma_d, bila_sigma_r),
        };
        Ok(Image::Normal(dest))
    }
}

/// Kernel:
/// K(x, y) = A * 1
fn box_filter_kernel(kernel_size: usize) -> Array2<f32> {
    // Normalize factor.
    let k = 1.0 / kernel_size.pow(2) as f32;
    Array::from_elem((kernel_size, kernel_size), k)
}

/// Kernel:
/// G(x, y) = A e^((-x^2-y^2)/σ^2)
fn gauss_filter_kernel(kernel_size: usize, sigma: f32) -> Array2<f32> {
    let mid = (kernel_size / 2) as f32;
    let mut kernel = Array::from_shape_fn((kernel_size, kernel_size), |(x, y)| {
        let (x, y) = (x as f32, y as f32);
        ((-(x - mid).powi(2) - (y - mid).powi(2)) / sigma.powi(2)).exp()
    });
    // Normalize.
    kernel /= kernel.sum();
    kernel
}

fn linear_filter(src: Array3<f32>, kernel: Array2<f32>) -> Array3<f32> {
    let (ksize, ksize_) = kernel.dim();
    assert_eq!(ksize, ksize_);
    assert!(ksize > 0 && ksize % 2 == 1);

    let (h, w, ncol) = src.dim();
    assert_eq!(ncol, 3);
    let (h2, w2) = (h - ksize, w - ksize);
    let mut dest = Array::zeros((h2, w2, 3));
    Zip::indexed(&mut dest).par_apply(|(x, y, col), v| {
        *v = (&src.slice(s![x..x + ksize, y..y + ksize, col]) * &kernel).sum();
    });

    dest
}

/// https://bokjan.com/2018/11/lab-digital-image-processing.html#menu_index_19
fn wiener_filter(src: Array3<f32>, neighbor: usize) -> Array3<f32> {
    let (h, w, ncol) = src.dim();
    assert_eq!(ncol, 3);
    assert!(neighbor <= h && neighbor <= w);
    let (h2, w2) = (h - neighbor, w - neighbor);

    let mut mean = Array::zeros((h2, w2, 3));
    Zip::indexed(&mut mean).par_apply(|(x, y, col), v| {
        *v = src.slice(s![x..x + neighbor, y..y + neighbor, col]).sum()
            / (neighbor * neighbor) as f32;
    });

    let mut dev = Array::zeros((h2, w2, 3));
    Zip::indexed(&mut dev).par_apply(|(x, y, col), v| {
        let m = &src.slice(s![x..x + neighbor, y..y + neighbor, col])
            - &ArrayView::from(&[mean[[x, y, col]]]);
        *v = (&m * &m).sum() / (neighbor * neighbor) as f32;
    });

    let nu2 = dev.sum() / (h * w) as f32;

    let mut dest = Array::zeros((h2, w2, 3));
    Zip::indexed(&mut dest).par_apply(|(x, y, col), v| {
        let (mean, dev) = (mean[[x, y, col]], dev[[x, y, col]]);
        *v = mean + (dev - nu2).max(0.) / dev.max(nu2) * (src[[x, y, col]] - mean);
    });

    dest
}

fn bilateral_filter(src: Array3<f32>, neighbor: usize, sigma_d: f32, sigma_r: f32) -> Array3<f32> {
    let (h, w, ncol) = src.dim();
    assert_eq!(ncol, 3);
    assert!(neighbor <= h && neighbor <= w);
    let (h2, w2) = (h - neighbor, w - neighbor);
    let mid = neighbor / 2;

    let mut dest = Array::zeros((h2, w2, 3));
    Zip::indexed(&mut dest).par_apply(|(x, y, col), v| {
        let (mut sum, mut wsum) = (0.0, 0.0);
        for i in 0..neighbor {
            for j in 0..neighbor {
                let dd = ((i as f32 - mid as f32).powi(2) + (j as f32 - mid as f32).powi(2))
                    / (2.0 * sigma_d.powi(2));
                let dr = (src[[x + i, y + j, col]] - src[[x + mid, y + mid, col]])
                    .abs()
                    .powi(2)
                    / (2.0 * sigma_r.powi(2));
                let w = (-dd - dr).exp();
                wsum += w;
                sum += src[[x + i, y + j, col]] * w;
            }
        }
        *v = sum / wsum;
    });

    dest
}
