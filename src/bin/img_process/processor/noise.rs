use crate::util::{BuilderExtManualExt as _, Image, Result};
use gtk::{prelude::*, Builder};
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
                run(Box::new((mu as f32, sigma as f32)))
            })),
            _ => None,
        }
    }

    // Now only gaussion noise is implemented.
    fn run(&self, args: Box<dyn Any + Send>, src: Image) -> Result<Image> {
        use rand::prelude::*;
        let (mu, sigma): (f32, f32) = *args.downcast_ref().unwrap();

        let mut mat = src.expect_normal()?;

        let mut rng = rand::thread_rng();
        let gauss = rand_distr::Normal::new(mu, sigma.max(0.0)).unwrap();
        for v in mat.iter_mut() {
            *v += gauss.sample(&mut rng);
        }

        Ok(Image::Normal(mat))
    }
}
