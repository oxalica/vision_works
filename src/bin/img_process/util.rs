use failure::{bail, ensure, format_err, Error};
use gdk_pixbuf::{Colorspace, Pixbuf};
use glib::{IsA, Object};
use gtk::prelude::BuilderExtManual;
use ndarray::prelude::*;
use num_complex::Complex32 as C;
use std::fmt::Display;

pub type Result<T> = std::result::Result<T, Error>;

pub trait OptionExt<T> {
    fn context(self, context: impl Display + Send + Sync + 'static) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn context(self, context: impl Display + Send + Sync + 'static) -> Result<T> {
        self.ok_or_else(|| format_err!("{}", context))
    }
}

pub trait BuilderExtManualExt {
    fn object<T: IsA<Object>>(&self, name: &str) -> T;
}

impl<U: BuilderExtManual> BuilderExtManualExt for U {
    fn object<T: IsA<Object>>(&self, name: &str) -> T {
        self.get_object(name).unwrap_or_else(|| {
            panic!(
                "Missing object `{}` of type `{}`",
                name,
                std::any::type_name::<T>(),
            );
        })
    }
}

/// The image to be processed and rendered.
#[derive(Debug, Clone)]
pub enum Image {
    Normal(Array3<f32>), // [h, w, <rgb>]
    Complex(Array2<C>),
}

impl Image {
    pub fn open(path: &std::path::Path) -> Result<(Self, Pixbuf)> {
        let pixbuf = Pixbuf::new_from_file(path)?;
        let (h, w) = (pixbuf.get_height() as usize, pixbuf.get_width() as usize);
        ensure!(
            pixbuf.get_bits_per_sample() == 8 && pixbuf.get_colorspace() == Colorspace::Rgb,
            "Only 24-bit RGB colorspace is supported",
        );

        let pixels = unsafe { pixbuf.get_pixels() as &[u8] };
        let row_stride = pixbuf.get_rowstride() as usize;
        let mut mat = Array::zeros((h, w, 3));
        for x in 0..h {
            for y in 0..w {
                let idx = x * row_stride + y * 3;
                let (r, g, b) = (pixels[idx], pixels[idx + 1], pixels[idx + 2]);
                mat[[x, y, 0]] = r as f32 / 256.0;
                mat[[x, y, 1]] = g as f32 / 256.0;
                mat[[x, y, 2]] = b as f32 / 256.0;
            }
        }
        Ok((Image::Normal(mat), pixbuf))
    }

    pub fn expect_normal(self) -> Result<Array3<f32>> {
        match self {
            Self::Normal(img) => Ok(img),
            Self::Complex(_) => bail!("Expecting a normal image, found complex image"),
        }
    }

    pub fn expect_complex(self) -> Result<Array2<C>> {
        match self {
            Self::Complex(img) => Ok(img),
            Self::Normal(_) => bail!("Expecting a complex image, found normal image"),
        }
    }

    fn to_pixels(&self) -> Array3<u8> {
        match self {
            Self::Normal(mat) => {
                let (h, w, ncol) = mat.dim();
                assert_eq!(ncol, 3);
                let mut pixels = Array::zeros((h, w, 3));
                for ((x, y, col), v) in mat.indexed_iter() {
                    pixels[[x, y, col]] = (v * 256.0).max(0.0).min(255.0) as u8;
                }
                pixels
            }

            // Render grayscale `log(norm^2(value) + 1)` with normalization.
            Self::Complex(mat) => {
                let (h, w) = mat.dim();

                // Normalize factor.
                let scale = mat
                    .iter()
                    .map(|v| v.norm_sqr().ln_1p())
                    .max_by(|a, b| a.partial_cmp(&b).unwrap())
                    .unwrap_or(1.0);

                let mut pixels = Array::zeros((h, w, 3));
                for ((x, y), v) in mat.indexed_iter() {
                    let v = v.norm_sqr().ln_1p() / scale;
                    let gray = (v * 256.0).max(0.0).min(255.0) as u8;
                    pixels[[x, y, 0]] = gray;
                    pixels[[x, y, 1]] = gray;
                    pixels[[x, y, 2]] = gray;
                }
                pixels
            }
        }
    }

    pub fn render(&self) -> Pixbuf {
        let pixels = self.to_pixels();
        let (h, w, ncol) = pixels.dim();
        assert_eq!(ncol, 3);
        let raw_pixels = pixels.into_raw_vec();
        assert_eq!(raw_pixels.len(), h * w * ncol, "Should have no row align");
        Pixbuf::new_from_mut_slice(
            raw_pixels,
            Colorspace::Rgb,
            false,
            8,
            w as _,
            h as _,
            w as i32 * 3,
        )
    }
}
