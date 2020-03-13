use crate::{ext::BuilderExtManualExt as _, Result};
use gtk::{prelude::*, Builder};
use opencv::prelude::*;
use std::any::Any;

pub struct LinearTransform;

impl super::ImageProcessor for LinearTransform {
    fn register_handler(
        &self,
        builder: &Builder,
        handler_name: &str,
        run: Box<dyn Fn(Box<dyn Any + Send>) + 'static>,
    ) -> Option<Box<dyn Fn() + 'static>> {
        let builder = builder.clone();
        match handler_name {
            "on_linear_trans_reset" => Some(Box::new(move || {
                let scale: gtk::Scale = builder.object("scl_linear_trans_scale");
                let rotate: gtk::Scale = builder.object("scl_linear_trans_rotate");
                scale.set_value(1.0);
                rotate.set_value(0.0);
            })),
            "on_linear_trans_run" => Some(Box::new(move || {
                let scale: gtk::Scale = builder.object("scl_linear_trans_scale");
                let rotate: gtk::Scale = builder.object("scl_linear_trans_rotate");
                let scale = scale.get_value();
                let rotate = rotate.get_value();
                log!("Linear transform with scale={} rotate={}", scale, rotate);
                run(Box::new((scale, rotate)));
            })),
            _ => None,
        }
    }

    fn run(&self, args: Box<dyn Any + Send>, src: Mat) -> Result<Mat> {
        let (scale, rotate): (f64, f64) = *args.downcast_ref().unwrap();
        // todo!()
        Ok(Mat::copy(&src)?)
    }
}
