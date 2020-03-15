use crate::{ext::BuilderExtManualExt as _, Result};
use gtk::{prelude::*, Builder};
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
                run(Box::new((mu, sigma)))
            })),
            _ => None,
        }
    }

    // Now only gaussion noise is implemented.
    fn run(&self, args: Box<dyn Any + Send>, src: Mat) -> Result<Mat> {
        use opencv::core::*;
        use rand::prelude::*;

        let (mu, sigma): (f64, f64) = *args.downcast_ref().unwrap();
        let mut rng = rand::thread_rng();
        let gauss = rand_distr::Normal::new(mu, sigma.max(0.0)).unwrap();

        let (h, w) = (src.rows(), src.cols());
        let mut dest = src.clone()?;
        let mut apply_gauss = |x: &mut u8| {
            let dt = gauss.sample(&mut rng);
            *x = (*x as f64 + dt).max(0.0).min(255.0) as u8;
        };
        for x in 0..h {
            for y in 0..w {
                let [r, g, b] = &mut dest.at_2d_mut::<Vec3b>(x, y)?.0;
                apply_gauss(r);
                apply_gauss(g);
                apply_gauss(b);
            }
        }

        Ok(dest)
    }
}
