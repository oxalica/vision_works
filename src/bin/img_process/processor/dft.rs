use crate::util::{Image, Result};
use gtk::Builder;
use ndarray::prelude::*;
use num_complex::Complex32 as C;
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

    fn run(&self, args: Box<dyn Any + Send>, src: Image) -> Result<Image> {
        let inverse: bool = *args.downcast_ref().unwrap();
        if !inverse {
            let src = src.expect_normal()?;
            let (h, w, _) = src.dim();

            // Convert to complex grayscale image.
            let mut src_gray = Array::zeros((h, w));
            for ((x, y), v) in src_gray.indexed_iter_mut() {
                let (r, g, b) = (src[[x, y, 0]], src[[x, y, 1]], src[[x, y, 2]]);
                let mut gray = 0.299 * b as f32 + 0.587 * g as f32 + 0.114 * r as f32;
                // FFT shift
                if (x + y) % 2 == 1 {
                    gray = -gray;
                }
                *v = gray.into();
            }

            let dest_comp = fft_2d(src_gray, false);
            Ok(Image::Complex(dest_comp))
        } else {
            let src_comp = src.expect_complex()?;
            let dest_comp = fft_2d(src_comp, true);

            let (h, w) = dest_comp.dim();
            // FIXME: Change to `Array::from_shape_fn`
            let mut dest = Array::zeros((h, w, 3));
            for x in 0..h {
                for y in 0..w {
                    let gray = dest_comp[[x, y]].norm();
                    dest[[x, y, 0]] = gray;
                    dest[[x, y, 1]] = gray;
                    dest[[x, y, 2]] = gray;
                }
            }
            Ok(Image::Normal(dest))
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
