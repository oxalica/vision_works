use crate::util::{BuilderExtManualExt as _, Image, Result};
use gtk::{prelude::*, Builder};
use ndarray::prelude::*;
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
                builder
                    .object::<gtk::Scale>("scl_affine_trans_scale")
                    .set_value(1.0);
                builder
                    .object::<gtk::Scale>("scl_affine_trans_rotate")
                    .set_value(0.0);
            })),
            "on_affine_trans_run" => Some(Box::new(move || {
                let scale: gtk::Scale = builder.object("scl_affine_trans_scale");
                let rotate: gtk::Scale = builder.object("scl_affine_trans_rotate");
                let scale = scale.get_value() as f32;
                let rotate = rotate.get_value() as f32;
                run(Box::new((scale, rotate)));
            })),
            _ => None,
        }
    }

    fn run(&self, args: Box<dyn Any + Send>, src: Image) -> Result<Image> {
        let (scale, rotate): (f32, f32) = *args.downcast_ref().unwrap();
        let rotate = rotate.to_radians();
        let src = src.expect_normal()?;
        let dest = affine_trans(src, scale, rotate);
        Ok(Image::Normal(dest))
    }
}

fn affine_trans(src: Array3<f32>, scale: f32, rotate: f32) -> Array3<f32> {
    let (h, w, _) = src.dim();
    let (h2, w2) = get_size_after_affine_trans(h, w, scale, rotate);

    // Inverse matrix. So we can get source points for each destination points.
    let inv_trans_mat = get_translate_mat(h as f32 / 2.0, w as f32 / 2.0)
        .dot(&get_rotation_mat(-rotate))
        .dot(&get_scale_mat(1.0 / scale))
        .dot(&get_translate_mat(-(h2 as f32 / 2.0), -(w2 as f32 / 2.0)));

    let mut dest = Array::zeros((h2, w2, 3));
    for ((dest_x, dest_y, col), v) in dest.indexed_iter_mut() {
        // Slow: Result matrix is on heap.
        // let src_pt = inv_trans_mat.dot(&array![[dest_x as f32], [dest_y as f32], [1.]]);
        // let (x, y) = (src_pt[[0, 0]], src_pt[[1, 0]]);

        let (dest_x, dest_y) = (dest_x as f32, dest_y as f32);
        let m = &inv_trans_mat;
        let x = m[[0, 0]] * dest_x + m[[0, 1]] * dest_y + m[[0, 2]];
        let y = m[[1, 0]] * dest_x + m[[1, 1]] * dest_y + m[[1, 2]];

        // Top-left corner of neighborhood.
        let (x_, y_) = (x.floor() as isize - 1, y.floor() as isize - 1);

        let mut xsamples = [0.0; 4];
        for i in 0..4 {
            let mut ysamples = [0.0; 4];
            for j in 0..4 {
                // Treat neighbor pixels out of image as black.
                if 0 <= x_ + i && x_ + i < h as _ && 0 <= y_ + j && y_ + j < w as _ {
                    ysamples[j as usize] = src[[(x_ + i) as _, (y_ + j) as _, col]];
                }
            }
            xsamples[i as usize] = interpolate3(ysamples, y - y_ as f32);
        }
        *v = interpolate3(xsamples, x - x_ as f32);
    }

    dest
}

fn get_size_after_affine_trans(h: usize, w: usize, scale: f32, rotate: f32) -> (usize, usize) {
    let rot_mat = get_rotation_mat(rotate).dot(&get_scale_mat(scale));
    // Transform two border points to locate the result rectangle.
    let p1 = rot_mat.dot(&array![[h as f32], [w as f32], [1.]]);
    let p2 = rot_mat.dot(&array![[h as f32], [-(w as f32)], [1.]]);
    let h2 = p1[[0, 0]].abs().max(p2[[0, 0]].abs());
    let w2 = p1[[1, 0]].abs().max(p2[[1, 0]].abs());
    (h2.round() as usize, w2.round() as usize)
}

#[rustfmt::skip]
fn get_rotation_mat(th: f32) -> Array2<f32> {
    let (s, c) = (th.sin(), th.cos());
    array![
        [ c, -s , 0.],
        [ s,  c , 0.],
        [ 0., 0., 1.],
    ]
}

#[rustfmt::skip]
fn get_translate_mat(x: f32, y: f32) -> Array2<f32> {
    array![
        [ 1., 0., x ],
        [ 0., 1., y ],
        [ 0., 0., 1.],
    ]
}

#[rustfmt::skip]
fn get_scale_mat(scale: f32) -> Array2<f32> {
    array![
        [ scale,    0., 0.],
        [    0., scale, 0.],
        [    0.,    0., 1.],
    ]
}

#[rustfmt::skip]
fn interpolate3(y: [f32; 4], tx: f32) -> f32 {
    let x = [0.0, 1.0, 2.0, 3.0];
      (tx - x[1]) * (tx - x[2]) * (tx - x[3]) / ((x[0] - x[1]) * (x[0] - x[2]) * (x[0] - x[3])) * y[0]
    + (tx - x[0]) * (tx - x[2]) * (tx - x[3]) / ((x[1] - x[0]) * (x[1] - x[2]) * (x[1] - x[3])) * y[1]
    + (tx - x[0]) * (tx - x[1]) * (tx - x[3]) / ((x[2] - x[0]) * (x[2] - x[1]) * (x[2] - x[3])) * y[2]
    + (tx - x[0]) * (tx - x[1]) * (tx - x[2]) / ((x[3] - x[0]) * (x[3] - x[1]) * (x[3] - x[2])) * y[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_mat() {
        let pt1 = array![[3.0f32.sqrt()], [1.0], [1.0]];
        let pt2 = array![[0.0], [2.0], [1.0]];
        let dt = get_rotation_mat(std::f32::consts::PI / 3.0).dot(&pt1) - pt2;
        assert!(dt.iter().all(|x| x.abs() < 1e-5));
    }

    #[test]
    fn test_interpolate3() {
        assert!((interpolate3([0.0, 1.0, 2.0, 3.0], 1.5) - 1.5).abs() < 1e-5);
        assert!((interpolate3([0.0, 1.0, 4.0, 9.0], -2.0) - 4.0).abs() < 1e-5);
        assert!((interpolate3([0.0, 1.0, 8.0, 27.0], -4.0) - -64.0).abs() < 1e-5);
    }
}
