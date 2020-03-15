use crate::Result;
use gtk::Builder;
use opencv::prelude::Mat;
use std::{any::Any, sync::Arc};

mod affine_transform;
mod dft;
mod filter;
mod noise;

pub trait ImageProcessor: Send + Sync {
    fn register_handler(
        &self,
        builder: &Builder,
        handler_name: &str,
        run: Box<dyn Fn(Box<dyn Any + Send>) + 'static>,
    ) -> Option<Box<dyn Fn() + 'static>>;

    fn run(&self, args: Box<dyn Any + Send>, src: Mat) -> Result<Mat>;
}

pub fn load_processors() -> Vec<Arc<dyn ImageProcessor>> {
    vec![
        Arc::new(affine_transform::AffineTransform),
        Arc::new(dft::DFT),
        Arc::new(noise::Noise),
        Arc::new(filter::Filter),
    ]
}
