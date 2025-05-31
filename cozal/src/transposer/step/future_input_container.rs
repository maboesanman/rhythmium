use std::collections::BTreeSet;

use archery::SharedPointerKind;

use crate::transposer::Transposer;

use super::sub_step::boxed_input::BoxedInput;

/// A trait for a container that can be used to retrieve the next input to be processed.
///
/// This is usually going to be a `BTreeSet<BoxedInput>`, but it can be any type that implements this trait.
pub trait FutureInputContainer<'t, T: Transposer + 't, P: SharedPointerKind + 't>: 't {
    /// Get a guard for the next input to be processed.
    fn peek_time(&self) -> Option<T::Time>;

    /// Get the next input to be processed.
    fn take_next(&mut self) -> Option<BoxedInput<'t, T, P>>;
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for Option<BoxedInput<'t, T, P>>
{
    fn peek_time(&self) -> Option<T::Time> {
        self.as_ref().map(|i| i.get_time())
    }

    fn take_next(&mut self) -> Option<BoxedInput<'t, T, P>> {
        self.take()
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for BTreeSet<BoxedInput<'t, T, P>>
{
    fn peek_time(&self) -> Option<T::Time> {
        self.first().map(|i| i.get_time())
    }

    fn take_next(&mut self) -> Option<BoxedInput<'t, T, P>> {
        self.pop_first()
    }
}
