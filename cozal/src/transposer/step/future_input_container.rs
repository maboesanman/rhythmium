use std::collections::BTreeSet;

use archery::SharedPointerKind;

use crate::transposer::Transposer;

use super::sub_step::boxed_input_sub_step::BoxedInputSubStep;

pub trait FutureInputContainer<'t, T: Transposer + 't, P: SharedPointerKind + 't>: 't {
    fn next(&'_ mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_>;
}

pub trait FutureInputContainerGuard<'t, T: Transposer, P: SharedPointerKind>: Sized {
    fn get_time(&self) -> T::Time;
    fn take_sub_step(self) -> (BoxedInputSubStep<'t, T, P>, Option<Self>);
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for Option<BoxedInputSubStep<'t, T, P>>
{
    fn next(&mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_> {
        match self {
            Some(_) => Some(self),
            None => None,
        }
    }
}

impl<'t, T: Transposer, P: SharedPointerKind> FutureInputContainerGuard<'t, T, P>
    for &mut Option<BoxedInputSubStep<'t, T, P>>
{
    fn get_time(&self) -> T::Time {
        self.as_ref().unwrap().get_time()
    }

    fn take_sub_step(self) -> (BoxedInputSubStep<'t, T, P>, Option<Self>) {
        (core::mem::take(self).unwrap(), None)
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for BTreeSet<BoxedInputSubStep<'t, T, P>>
{
    fn next(&mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl<'t, T: Transposer, P: SharedPointerKind> FutureInputContainerGuard<'t, T, P>
    for &mut BTreeSet<BoxedInputSubStep<'t, T, P>>
{
    fn get_time(&self) -> T::Time {
        self.first().unwrap().get_time()
    }

    fn take_sub_step(self) -> (BoxedInputSubStep<'t, T, P>, Option<Self>) {
        let value = self.pop_first().unwrap();
        let next = if self.is_empty() { None } else { Some(self) };

        (value, next)
    }
}
