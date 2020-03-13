use failure::{format_err, Error};
use std::fmt::Display;

pub trait OptionExt<T> {
    fn context(self, context: impl Display + Send + Sync + 'static) -> Result<T, Error>;
}

impl<T> OptionExt<T> for Option<T> {
    fn context(self, context: impl Display + Send + Sync + 'static) -> Result<T, Error> {
        self.ok_or_else(|| format_err!("{}", context))
    }
}
