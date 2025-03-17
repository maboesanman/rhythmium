use std::collections::BTreeSet;

use archery::SharedPointerKind;

use crate::transposer::Transposer;

use super::sub_step::boxed_input::BoxedInput;

/// A trait for a container that can be used to retrieve the next input to be processed.
///
/// This is usually going to be a `BTreeSet<BoxedInput>`, but it can be any type that implements this trait.
pub trait FutureInputContainer<'t, T: Transposer + 't, P: SharedPointerKind + 't>: 't {
    /// Get a guard for the next input to be processed.
    fn next(&'_ mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_>;
}

/// A trait for a guard that can be used to retrieve the next input to be processed.
///
/// This guard should remove the input from the container when `take_sub_step` is called,
/// and should leave it in the container when dropped.
pub trait FutureInputContainerGuard<'t, T: Transposer, P: SharedPointerKind>: Sized {
    /// Get the time of the input.
    fn get_time(&self) -> T::Time;

    /// Take the input from the container.
    ///
    /// # Returns
    ///
    /// A tuple containing the input that was taken, and an optional guard for the next input to be processed.
    fn take_sub_step(self) -> (BoxedInput<'t, T, P>, Option<Self>);
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for Option<BoxedInput<'t, T, P>>
{
    fn next(&mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_> {
        match self {
            Some(_) => Some(self),
            None => None,
        }
    }
}

impl<'t, T: Transposer, P: SharedPointerKind> FutureInputContainerGuard<'t, T, P>
    for &mut Option<BoxedInput<'t, T, P>>
{
    fn get_time(&self) -> T::Time {
        self.as_ref().unwrap().get_time()
    }

    fn take_sub_step(self) -> (BoxedInput<'t, T, P>, Option<Self>) {
        (core::mem::take(self).unwrap(), None)
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for BTreeSet<BoxedInput<'t, T, P>>
{
    fn next(&mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_> {
        if self.is_empty() { None } else { Some(self) }
    }
}

impl<'t, T: Transposer, P: SharedPointerKind> FutureInputContainerGuard<'t, T, P>
    for &mut BTreeSet<BoxedInput<'t, T, P>>
{
    fn get_time(&self) -> T::Time {
        self.first().unwrap().get_time()
    }

    fn take_sub_step(self) -> (BoxedInput<'t, T, P>, Option<Self>) {
        let value = self.pop_first().unwrap();
        let next = if self.is_empty() { None } else { Some(self) };

        (value, next)
    }
}
