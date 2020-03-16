use crate::{ext::BuilderExtManualExt as _, Result};
use gtk::{prelude::*, Builder};
use ndarray::prelude::*;
use opencv::prelude::*;
use std::any::Any;

pub struct Noise;

impl super::ImageProcessor for Noise {
    fn register_handler(
        &self,
        builder: &Builder,
        handler_name: &str,
        run: Box<dyn Fn(Box<dyn Any + Send>) + 'static>,
    ) -> Option<Box<dyn Fn() + 'static>> {
        let builder = builder.clone();
        match handler_name {
            "on_noise_gauss_reset" => Some(Box::new(move || {
                builder
                    .object::<gtk::Scale>("scl_noise_gauss_mu")
                    .set_value(0.0);
                builder
                    .object::<gtk::Scale>("scl_noise_gauss_sigma")
                    .set_value(0.0);
            })),
            "on_noise_gauss_run" => Some(Box::new(move || {
                let mu = builder
                    .object::<gtk::Scale>("scl_noise_gauss_mu")
                    .get_value();
                let sigma = builder
                    .object::<gtk::Scale>("scl_noise_gauss_sigma")
                    .get_value();
                run(Box::new((mu as f32 / 256.0, sigma as f32 / 256.0)))
            })),
            _ => None,
        }
    }

    // Now only gaussion noise is implemented.
    fn run(&self, args: Box<dyn Any + Send>, src: Mat) -> Result<Mat> {
        use opencv::core::{Scalar, Vec3b, CV_8UC3};
        use rand::prelude::*;
        let (mu, sigma): (f32, f32) = *args.downcast_ref().unwrap();

        let (h, w) = (src.rows() as usize, src.cols() as usize);
        let mut mat = Array::zeros((h, w, 3));
        for x in 0..h {
            for y in 0..w {
                let [b, g, r] = src.at_2d::<Vec3b>(x as _, y as _).unwrap().0;
                mat[[x, y, 0]] = r as f32 / 256.0;
                mat[[x, y, 1]] = g as f32 / 256.0;
                mat[[x, y, 2]] = b as f32 / 256.0;
            }
        }

        let mut rng = rand::thread_rng();
        let gauss = rand_distr::Normal::new(mu, sigma.max(0.0)).unwrap();
        for v in mat.iter_mut() {
            *v += gauss.sample(&mut rng);
        }

        let mut dest = Mat::new_rows_cols_with_default(h as _, w as _, CV_8UC3, Scalar::all(0.0))?;
        for x in 0..h {
            for y in 0..w {
                let (r, g, b) = (mat[[x, y, 0]], mat[[x, y, 1]], mat[[x, y, 2]]);
                dest.at_2d_mut::<Vec3b>(x as _, y as _).unwrap().0 = [
                    (b * 256.0).max(0.0).min(255.0) as u8,
                    (g * 256.0).max(0.0).min(255.0) as u8,
                    (r * 256.0).max(0.0).min(255.0) as u8,
                ];
            }
        }

        Ok(dest)
    }
}
