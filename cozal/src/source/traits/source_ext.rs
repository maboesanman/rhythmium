use super::Source;
// use crate::adapters::MutexSource;

impl<S> SourceExt for S where S: Source {}

pub trait SourceExt: Source + Sized {}
