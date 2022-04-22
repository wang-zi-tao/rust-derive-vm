use failure::Fallible;
use std::{iter::Peekable, slice::Iter};

pub type _Iter<'a, T> = Peekable<Iter<'a, T>>;
pub type _Fallible<T> = Fallible<T>;
pub type _Vec<T> = Vec<T>;
pub use failure::format_err as _format_err;
pub use std::{default::Default as _Default, unreachable as _unreachable};
