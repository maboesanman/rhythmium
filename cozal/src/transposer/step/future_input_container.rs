use std::collections::BTreeSet;

use archery::SharedPointerKind;

use crate::transposer::Transposer;

use super::sub_step::BoxedSubStep;

pub trait FutureInputContainer<'t, T: Transposer + 't, P: SharedPointerKind + 't>: 't {
    fn next(&'_ mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_>;
}

pub trait FutureInputContainerGuard<'t, T: Transposer, P: SharedPointerKind> {
    fn get_time(&self) -> T::Time;
    fn take_sub_step(self) -> BoxedSubStep<'t, T, P>;
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for Option<BoxedSubStep<'t, T, P>>
{
    fn next(&mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_> {
        match self {
            Some(_) => Some(self),
            None => None,
        }
    }
}

impl<'a, 't, T: Transposer, P: SharedPointerKind> FutureInputContainerGuard<'t, T, P>
    for &'a mut Option<BoxedSubStep<'t, T, P>>
{
    fn get_time(&self) -> T::Time {
        self.as_ref().unwrap().as_ref().get_time()
    }

    fn take_sub_step(self) -> BoxedSubStep<'t, T, P> {
        core::mem::take(self).unwrap()
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> FutureInputContainer<'t, T, P>
    for BTreeSet<BoxedSubStep<'t, T, P>>
{
    fn next(&mut self) -> Option<impl FutureInputContainerGuard<'t, T, P> + '_> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl<'a, 't, T: Transposer, P: SharedPointerKind> FutureInputContainerGuard<'t, T, P>
    for &'a mut BTreeSet<BoxedSubStep<'t, T, P>>
{
    fn get_time(&self) -> T::Time {
        self.first().unwrap().as_ref().get_time()
    }

    fn take_sub_step(self) -> BoxedSubStep<'t, T, P> {
        self.pop_first().unwrap()
    }
}
