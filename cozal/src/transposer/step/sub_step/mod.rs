use std::{
    any::TypeId,
    cell::UnsafeCell,
    cmp::Ordering,
    pin::Pin,
    ptr::NonNull,
    task::{Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    input_state_manager::InputStateManager, output_event_manager::OutputEventManager, Transposer,
};

use super::wrapped_transposer::WrappedTransposer;

pub mod init_sub_step;
pub mod input_sub_step;
pub mod scheduled_sub_step;

pub trait SubStep<T: Transposer, P: SharedPointerKind> {
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
    fn is_unsaturated(&self) -> bool;
    fn is_saturating(&self) -> bool;
    fn is_saturated(&self) -> bool;
    fn get_time(&self) -> T::Time;

    fn cmp(&self, other: &dyn SubStep<T, P>) -> Ordering;

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
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PollErr {
    NotSaturating,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum StartSaturateErr {
    SubStepTimeIsPast,
    NotUnsaturated,
}
