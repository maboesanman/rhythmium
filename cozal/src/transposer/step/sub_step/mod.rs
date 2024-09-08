use std::{any::{Any, TypeId}, cell::UnsafeCell, cmp::Ordering, future::Future, marker::PhantomData, pin::Pin, ptr::NonNull, task::{Poll, Waker}};

use archery::{ArcTK, SharedPointer, SharedPointerKind};

use crate::transposer::Transposer;

use super::{wrapped_transposer::WrappedTransposer, InputState, StepPoll};

pub mod init_sub_step;
pub mod input_sub_step;
pub mod scheduled_sub_step;

pub trait SubStep<T: Transposer, P: SharedPointerKind, S: InputState<T>> {
    fn is_input(&self) -> bool { false }
    fn input_sort(&self) -> Option<(u64, TypeId)> { None }
    fn is_init(&self) -> bool { false }
    fn is_scheduled(&self) -> bool { false }
    fn is_unsaturated(&self) -> bool;
    fn is_saturating(&self) -> bool;
    fn is_saturated(&self) -> bool;
    fn get_time(&self) -> T::Time;

    fn cmp(&self, other: &dyn SubStep<T, P, S>) -> Ordering;

    fn start_saturate(
        self: Pin<&mut Self>,
        transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<UnsafeCell<S>>,
        outputs_to_swallow: usize,
    ) -> Result<(), StartSaturateErr>;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, PollErr>;
    
    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>>;
    
    fn take_finished_transposer(self: Pin<&mut Self>) -> Option<SharedPointer<WrappedTransposer<T, P>, P>>;
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
