use failure::{format_err, Error};
use glib::{IsA, Object};
use gtk::prelude::BuilderExtManual;
use std::fmt::Display;

pub trait OptionExt<T> {
    fn context(self, context: impl Display + Send + Sync + 'static) -> Result<T, Error>;
}

impl<T> OptionExt<T> for Option<T> {
    fn context(self, context: impl Display + Send + Sync + 'static) -> Result<T, Error> {
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
