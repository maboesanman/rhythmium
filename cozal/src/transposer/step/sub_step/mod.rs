use std::{
    any::TypeId,
    cmp::Ordering,
    pin::Pin,
    ptr::NonNull,
    task::{Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    input_state_manager::InputStateManager, output_event_manager::OutputEventManager, Transposer,
};

use super::{wrapped_transposer::WrappedTransposer, PollErr};

pub mod boxed_input_sub_step;
pub mod init_sub_step;
pub mod input_sub_step;
pub mod scheduled_sub_step;
pub const INIT_SUB_STEP_SORT_PHASE: usize = 1;
pub const INPUT_SUB_STEP_SORT_PHASE: usize = 2;
pub const SCHEDULED_SUB_STEP_SORT_PHASE: usize = 3;

#[repr(transparent)]
pub struct BoxedSubStep<'t, T: Transposer + 't, P: SharedPointerKind + 't>(
    Box<dyn SubStep<T, P> + 't>,
);

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> std::fmt::Debug for BoxedSubStep<'t, T, P>
where
    T::Time: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _type = if self.as_ref().is_input() {
            "input"
        } else if self.as_ref().is_init() {
            "init"
        } else {
            "scheduled"
        };
        let status = if self.as_ref().is_unsaturated() {
            "unsaturated"
        } else if self.as_ref().is_saturating() {
            "saturating"
        } else {
            "saturated"
        };
        f.debug_struct("BoxedSubStep")
            .field("time", &self.as_ref().get_time())
            .field("type", &_type)
            .field("status", &status)
            .finish()
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> BoxedSubStep<'t, T, P> {
    pub fn new(sub_step: Box<dyn SubStep<T, P> + 't>) -> Self {
        Self(sub_step)
    }

    pub fn as_ref(&self) -> &dyn SubStep<T, P> {
        &*self.0
    }

    pub fn as_mut(&mut self) -> Pin<&mut dyn SubStep<T, P>> {
        unsafe { Pin::new_unchecked(&mut *self.0) }
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> Ord for BoxedSubStep<'t, T, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().dyn_cmp(other.as_ref())
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> PartialOrd for BoxedSubStep<'t, T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> PartialEq for BoxedSubStep<'t, T, P> {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Equal)
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> Eq for BoxedSubStep<'t, T, P> {}

#[allow(dead_code)]
/// # Safety
/// This trait is unsafe because the dyn_cmp function must be implemented properly, or there could be UB.
pub unsafe trait SubStep<T: Transposer, P: SharedPointerKind> {
    fn is_input(&self) -> bool {
        false
    }
    fn input_sort(&self) -> Option<(u64, TypeId)> {
        None
    }
    fn is_init(&self) -> bool {
        false
    }
    fn is_scheduled(&self) -> bool {
        false
    }

    fn sort_phase(&self) -> usize;

    fn is_unsaturated(&self) -> bool;
    fn is_saturating(&self) -> bool;
    fn is_saturated(&self) -> bool;
    fn get_time(&self) -> T::Time;

    fn dyn_cmp(&self, other: &dyn SubStep<T, P>) -> Ordering;

    fn start_saturate(
        self: Pin<&mut Self>,
        transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> Result<(), StartSaturateErr>;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, PollErr>;

    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>>;

    fn take_finished_transposer(
        self: Pin<&mut Self>,
    ) -> Option<SharedPointer<WrappedTransposer<T, P>, P>>;

    fn desaturate(self: Pin<&mut Self>);
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum StartSaturateErr {
    SubStepTimeIsPast,
    NotUnsaturated,
}
