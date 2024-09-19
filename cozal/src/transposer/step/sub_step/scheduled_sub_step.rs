use std::{
    future::Future,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    input_state_manager::InputStateManager, output_event_manager::OutputEventManager,
    step::wrapped_transposer::WrappedTransposer, Transposer,
};

use super::{BoxedSubStep, PollErr, StartSaturateErr, SubStep};

#[allow(dead_code)]
pub fn new_scheduled_sub_step<T: Transposer + Clone, P: SharedPointerKind>(
    time: T::Time,
) -> impl SubStep<T, P> {
    ScheduledSubStepStatus::Unsaturated { time }
}

#[allow(dead_code)]
pub fn new_scheduled_boxed_sub_step<'a, T: Transposer + Clone + 'a, P: SharedPointerKind + 'a>(
    time: T::Time,
) -> BoxedSubStep<'a, T, P> {
    BoxedSubStep::new(Box::new(new_scheduled_sub_step::<T, P>(time)))
}

#[allow(unused)]
enum ScheduledSubStepStatus<T: Transposer + Clone, P: SharedPointerKind> {
    Unsaturated {
        time: T::Time,
    },
    Saturating {
        time: T::Time,
        future: wrapped_handler::WrappedHandlerFuture<T, P>,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    },
}

impl<T, P> SubStep<T, P> for ScheduledSubStepStatus<T, P>
where
    T: Transposer + Clone,
    P: SharedPointerKind,
{
    fn is_scheduled(&self) -> bool {
        true
    }
    fn is_unsaturated(&self) -> bool {
        matches!(self, ScheduledSubStepStatus::Unsaturated { .. })
    }

    fn is_saturating(&self) -> bool {
        matches!(self, ScheduledSubStepStatus::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, ScheduledSubStepStatus::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match self {
            ScheduledSubStepStatus::Unsaturated { time, .. } => *time,
            ScheduledSubStepStatus::Saturating { time, .. } => *time,
            ScheduledSubStepStatus::Saturated {
                wrapped_transposer, ..
            } => wrapped_transposer.metadata.last_updated.time,
        }
    }

    fn dyn_cmp(&self, other: &dyn SubStep<T, P>) -> std::cmp::Ordering {
        match other.is_scheduled() {
            true => std::cmp::Ordering::Equal,
            false => std::cmp::Ordering::Greater,
        }
    }

    fn start_saturate(
        self: Pin<&mut Self>,
        transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> Result<(), StartSaturateErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = match this {
            ScheduledSubStepStatus::Unsaturated { time, .. } => *time,
            _ => return Err(StartSaturateErr::NotUnsaturated),
        };

        if transposer.metadata.last_updated.time > time {
            return Err(StartSaturateErr::SubStepTimeIsPast);
        }

        let future = wrapped_handler::handle::<T, P>(transposer, time, shared_step_state);

        *this = ScheduledSubStepStatus::Saturating { time, future };

        Ok(())
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match this {
            ScheduledSubStepStatus::Unsaturated { .. } => return Err(PollErr::Unsaturated),
            ScheduledSubStepStatus::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            ScheduledSubStepStatus::Saturated { .. } => return Err(PollErr::Saturated),
        };

        *this = ScheduledSubStepStatus::Saturated { wrapped_transposer };

        Ok(Poll::Ready(()))
    }

    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>> {
        match self {
            ScheduledSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn take_finished_transposer(
        self: Pin<&mut Self>,
    ) -> Option<SharedPointer<WrappedTransposer<T, P>, P>> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        match core::mem::replace(this, ScheduledSubStepStatus::Unsaturated { time }) {
            ScheduledSubStepStatus::Unsaturated { .. } => None,
            ScheduledSubStepStatus::Saturating { .. } => None,
            ScheduledSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
        }
    }

    fn desaturate(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        *this = ScheduledSubStepStatus::Unsaturated { time };
    }
}

mod wrapped_handler {
    use super::*;

    pub type WrappedHandlerFuture<T: Transposer + Clone, P: SharedPointerKind> =
        impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>;

    pub fn handle<T: Transposer + Clone, P: SharedPointerKind>(
        mut wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        time: T::Time,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> WrappedHandlerFuture<T, P> {
        async move {
            let transposer_mut = SharedPointer::make_mut(&mut wrapped_transposer);
            transposer_mut
                .handle_scheduled(time, shared_step_state)
                .await;
            wrapped_transposer
        }
    }
}
