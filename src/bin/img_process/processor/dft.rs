use crate::Result;
use failure::ensure;
use gtk::Builder;
use ndarray::prelude::*;
use num_complex::Complex32 as C;
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
        use opencv::core::*;
        let inverse: bool = *args.downcast_ref().unwrap();
        if !inverse {
            ensure!(src.typ()? == CV_8UC3, "DFT input should be a normal image");

            // Convert to complex grayscale image.
            let (h, w) = (src.rows() as usize, src.cols() as usize);
            let mut mat: Array2<C> = Array::zeros((h, w));
            for ((x, y), p) in mat.indexed_iter_mut() {
                let [b, g, r] = src.at_2d::<Vec3b>(x as _, y as _).unwrap().0;
                let mut gray =
                    0.299 * b as f32 / 256.0 + 0.587 * g as f32 / 256.0 + 0.114 * r as f32 / 256.0;
                // FFT shift
                if (x + y) % 2 == 1 {
                    gray = -gray;
                }
                *p = gray.into();
            }

            let mat = fft_2d(mat, false);

            let (h2, w2) = mat.dim();
            let mut dest =
                Mat::new_rows_cols_with_default(h2 as _, w2 as _, CV_32FC2, Scalar::all(0.0))?;
            for ((x, y), v) in mat.indexed_iter() {
                dest.at_2d_mut::<Vec2f>(x as _, y as _).unwrap().0 = [v.re, v.im];
            }

            Ok(dest)
        } else {
            ensure!(
                src.typ()? == CV_32FC2,
                "Inverse-DFT input should be a complex matrix",
            );

            let (h, w) = (src.rows() as usize, src.cols() as usize);
            let mut mat = Array::zeros((h, w));
            for ((x, y), p) in mat.indexed_iter_mut() {
                let [re, im] = src.at_2d::<Vec2f>(x as _, y as _).unwrap().0;
                *p = C::new(re, im);
            }

            let mat = fft_2d(mat, true);

            let (h2, w2) = mat.dim();
            let mut dest =
                Mat::new_rows_cols_with_default(h2 as _, w2 as _, CV_8UC3, Scalar::all(0.0))?;
            for ((x, y), v) in mat.indexed_iter() {
                let gray = (v.norm() * 256.0).max(0.0).min(255.0) as u8;
                dest.at_2d_mut::<Vec3b>(x as _, y as _).unwrap().0 = [gray, gray, gray];
            }
            Ok(dest)
        }
    }
}

#[derive(Debug)]
struct FFT {
    w: Vec<C>,
    butterfly: Vec<usize>,
}

impl FFT {
    pub fn fft_size_of(n: usize) -> usize {
        n.next_power_of_two()
    }

    pub fn init(n: usize) -> Self {
        assert!(n.is_power_of_two());
        let n = Self::fft_size_of(n);
        let theta = 2.0 * std::f32::consts::PI / n as f32;
        let w = (0..n)
            .map(|i| C::from_polar(&1.0, &(theta * i as f32)))
            .collect();

        let mut butterfly = vec![0; n];
        let mut j = 0;
        for i in 1..n {
            let mut k = n >> 1;
            while j & k != 0 {
                k >>= 1;
            }
            j = j & (k - 1) | k;
            butterfly[i] = j;
        }

        Self { w, butterfly }
    }

    pub fn fft(&self, mut mat: ArrayViewMut1<C>, inverse: bool) {
        let n = self.w.len();
        let logn = n.trailing_zeros() as usize;
        assert_eq!(mat.shape(), [n]);

        // Butterfly swap
        (0..n)
            .filter(|&i| i < self.butterfly[i])
            .for_each(|i| mat.swap([i], [self.butterfly[i]]));

        let mut h = 1;
        for t in (0..logn).rev() {
            for i in (0..n).step_by(h << 1) {
                for j in 0..h {
                    let w = self.w[j << t];
                    let u = mat[[i + j]];
                    let v = mat[[i + j + h]] * if inverse { w.conj() } else { w };
                    mat[[i + j]] = u + v;
                    mat[[i + j + h]] = u - v;
                }
            }
            h <<= 1;
        }

        let k = (1.0 / n as f32).sqrt();
        mat.iter_mut().for_each(|x| *x *= k);
    }
}

// Gray image only.
pub fn fft_2d(src: Array2<C>, inverse: bool) -> Array2<C> {
    // Expand to FFT optimal size.
    let (n0, m0) = src.dim();
    let (n, m) = (FFT::fft_size_of(n0), FFT::fft_size_of(m0));
    let mut mat = Array::zeros((n, m));
    for (i, row) in src.outer_iter().enumerate() {
        mat.slice_mut(s![i, ..m0]).assign(&row);
    }

    let (f1, f2) = (FFT::init(n), FFT::init(m));
    if !inverse {
        // Run 1D-FFT for each row and then for each column.
        for mut row in mat.outer_iter_mut() {
            f2.fft(row.view_mut(), false);
        }
        let mut mat = mat.reversed_axes();
        for mut col in mat.outer_iter_mut() {
            f1.fft(col.view_mut(), false);
        }
        mat.reversed_axes()
    } else {
        // Run in inverse order when running inverse-FFT.
        let mut mat = mat.reversed_axes();
        for mut col in mat.outer_iter_mut() {
            f1.fft(col.view_mut(), true);
        }
        let mut mat = mat.reversed_axes();
        for mut row in mat.outer_iter_mut() {
            f2.fft(row.view_mut(), true);
        }
        mat
    }
}
