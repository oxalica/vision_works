use crate::{ext::BuilderExtManualExt as _, Result};
use gtk::{prelude::*, Builder};
use opencv::prelude::*;
use std::any::Any;

pub struct AffineTransform;

impl super::ImageProcessor for AffineTransform {
    fn register_handler(
        &self,
        builder: &Builder,
        handler_name: &str,
        run: Box<dyn Fn(Box<dyn Any + Send>) + 'static>,
    ) -> Option<Box<dyn Fn() + 'static>> {
        let builder = builder.clone();
        match handler_name {
            "on_affine_trans_reset" => Some(Box::new(move || {
                let scale: gtk::Scale = builder.object("scl_affine_trans_scale");
                let rotate: gtk::Scale = builder.object("scl_affine_trans_rotate");
                scale.set_value(1.0);
                rotate.set_value(0.0);
            })),
            "on_affine_trans_run" => Some(Box::new(move || {
                let scale: gtk::Scale = builder.object("scl_affine_trans_scale");
                let rotate: gtk::Scale = builder.object("scl_affine_trans_rotate");
                let scale = scale.get_value();
                let rotate = rotate.get_value();
                log!("Affine transform with scale={} rotate={}", scale, rotate);
                run(Box::new((scale, rotate)));
            })),
            _ => None,
        }
    }

    fn run(&self, args: Box<dyn Any + Send>, src: Mat) -> Result<Mat> {
        use opencv::{
            core::{Point2f, Scalar, BORDER_CONSTANT},
            imgproc::*,
        };

        let (scale, rotate): (f64, f64) = *args.downcast_ref().unwrap();
        let (h, w) = (src.rows() as f64, src.cols() as f64);
        let (s, c) = (rotate.to_radians().sin(), rotate.to_radians().cos());

        let mut trans_mat =
            get_rotation_matrix_2d(Point2f::new(w as f32 / 2.0, h as f32 / 2.0), rotate, scale)?;
        let (h2, w2) = (
            (c * h - s * w).abs().max((c * h + s * w).abs()) * scale,
            (s * h + c * w).abs().max((s * h - c * w).abs()) * scale,
        );
        *trans_mat.at_2d_mut::<f64>(0, 2).unwrap() += (w2 - w) / 2.0;
        *trans_mat.at_2d_mut::<f64>(1, 2).unwrap() += (h2 - h) / 2.0;

        let mut dest = Mat::new_rows_cols_with_default(
            h2.round() as _,
            w2.round() as _,
            src.typ()?,
            Scalar::all(0.0),
        )?;
        let dsize = dest.size()?;
        warp_affine(
            &src,
            &mut dest,
            &trans_mat,
            dsize,
            INTER_CUBIC,
            BORDER_CONSTANT,
            Scalar::all(0.0),
        )?;
        Ok(dest)
    }
}
