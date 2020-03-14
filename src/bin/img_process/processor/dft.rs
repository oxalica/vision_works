use crate::Result;
use failure::{ensure, ResultExt as _};
use gtk::Builder;
use opencv::prelude::*;
use std::any::Any;

pub struct DFT;

impl super::ImageProcessor for DFT {
    fn register_handler(
        &self,
        _builder: &Builder,
        handler_name: &str,
        run: Box<dyn Fn(Box<dyn Any + Send>) + 'static>,
    ) -> Option<Box<dyn Fn() + 'static>> {
        match handler_name {
            "on_dft_dft" => Some(Box::new(move || run(Box::new(false)))),
            "on_dft_idft" => Some(Box::new(move || run(Box::new(true)))),
            _ => None,
        }
    }

    fn run(&self, args: Box<dyn Any + Send>, src: Mat) -> Result<Mat> {
        let is_inv: bool = *args.downcast_ref().unwrap();
        if is_inv {
            idft(src)
        } else {
            dft(src)
        }
    }
}

fn dft(src: Mat) -> Result<Mat> {
    use opencv::{core::*, imgproc::*};

    // Convert to gray image first.
    let src_u8 = if src.typ()? == CV_8U {
        src
    } else {
        let mut gray = Mat::default()?;
        cvt_color(&src, &mut gray, COLOR_BGR2GRAY, 0)?;
        gray
    };

    // Extent matrix to optimal size for DFT.
    let h2 = get_optimal_dft_size(src_u8.rows())?;
    let w2 = get_optimal_dft_size(src_u8.cols())?;
    log!("DFT optimal size is {}x{}", h2, w2);

    // Copy to extended f32 matrix.
    let mut src_f32 = Mat::new_rows_cols_with_default(h2, w2, CV_32FC1, Scalar::all(0.0))?;
    for x in 0..h2 {
        for y in 0..w2 {
            let color = *src_u8.at_2d::<u8>(x, y).unwrap_or(&0);
            let mut color = color as f32 / 256.0;
            // Negate to make DFT produce low frequancy component in the middle of image
            // instead of corners.
            // See: https://bokjan.com/2018/11/lab-digital-image-processing.html
            color *= if (x + y) % 2 == 0 { 1.0 } else { -1.0 };
            *src_f32.at_2d_mut::<f32>(x, y).unwrap() = color;
        }
    }

    let mut dest_f32 = Mat::new_rows_cols_with_default(h2, w2, CV_32FC1, Scalar::all(0.0))?;
    dft(&src_f32, &mut dest_f32, DFT_COMPLEX_OUTPUT, 0).context("dft")?;
    Ok(dest_f32)
}

fn idft(src: Mat) -> Result<Mat> {
    use opencv::core::*;
    ensure!(
        src.typ()? == CV_32FC2,
        "IDFT input should be a complex matrix",
    );
    let (h, w) = (src.rows(), src.cols());

    let mut dest_comp = Mat::new_rows_cols_with_default(h, w, CV_32FC2, Scalar::all(0.0))?;
    idft(&src, &mut dest_comp, DFT_COMPLEX_OUTPUT, 0).context("idft")?;

    // Convert complex matrix to gray BGR.
    let mut dest = Mat::new_rows_cols_with_default(
        dest_comp.rows(),
        dest_comp.cols(),
        CV_8UC3,
        Scalar::all(0.0),
    )?;

    // Normalize and convert back to BGR (but gray).
    let mut mx = std::f32::EPSILON;
    for x in 0..dest_comp.rows() {
        for y in 0..dest_comp.cols() {
            let [a, b] = dest_comp.at_2d::<Vec2f>(x, y).unwrap().0;
            mx = mx.max(a.hypot(b));
        }
    }
    for x in 0..dest_comp.rows() {
        for y in 0..dest_comp.cols() {
            let [a, b] = dest_comp.at_2d::<Vec2f>(x, y).unwrap().0;
            // `hypot` eats the sign, so the negation done above will not affect the result.
            let gray = (a.hypot(b) / mx * 256.0) as u8;
            *dest.at_2d_mut::<Vec3b>(x, y).unwrap() = Vec3b::all(gray);
        }
    }
    Ok(dest)
}
