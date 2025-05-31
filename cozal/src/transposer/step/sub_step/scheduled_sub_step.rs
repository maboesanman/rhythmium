use std::{
    future::Future,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    Transposer, input_state_manager::InputStateManager, output_event_manager::OutputEventManager,
    step::wrapped_transposer::WrappedTransposer,
};

use super::{BoxedSubStep, PollErr, SCHEDULED_SUB_STEP_SORT_PHASE, StartSaturateErr, SubStep};

pub enum ScheduledSubStep<T: Transposer + Clone, P: SharedPointerKind> {
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

mod wrapped_handler {
    use super::*;

    pub type WrappedHandlerFuture<T: Transposer + Clone, P: SharedPointerKind> =
        impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>;

    #[define_opaque(WrappedHandlerFuture)]
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

impl<T, P> ScheduledSubStep<T, P>
where
    T: Transposer + Clone,
    P: SharedPointerKind,
{
    pub fn new(time: T::Time) -> Self {
        ScheduledSubStep::Unsaturated { time }
    }

    pub fn new_boxed<'a>(time: T::Time) -> BoxedSubStep<'a, T, P>
    where
        T: 'a,
        P: 'a,
    {
        BoxedSubStep::new(Box::new(Self::new(time)))
    }
}

unsafe impl<T, P> SubStep<T, P> for ScheduledSubStep<T, P>
where
    T: Transposer + Clone,
    P: SharedPointerKind,
{
    fn is_scheduled(&self) -> bool {
        true
    }

    fn is_unsaturated(&self) -> bool {
        matches!(self, ScheduledSubStep::Unsaturated { .. })
    }

    fn is_saturating(&self) -> bool {
        matches!(self, ScheduledSubStep::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, ScheduledSubStep::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match self {
            ScheduledSubStep::Unsaturated { time, .. } => *time,
            ScheduledSubStep::Saturating { time, .. } => *time,
            ScheduledSubStep::Saturated {
                wrapped_transposer, ..
            } => wrapped_transposer.metadata.last_updated.unwrap().time,
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
            ScheduledSubStep::Unsaturated { time, .. } => *time,
            _ => return Err(StartSaturateErr::NotUnsaturated),
        };

        if let Some(t) = transposer.metadata.last_updated
            && t.time > time
        {
            return Err(StartSaturateErr::SubStepTimeIsPast);
        }

        let future = wrapped_handler::handle::<T, P>(transposer, time, shared_step_state);

        *this = ScheduledSubStep::Saturating { time, future };

        Ok(())
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match this {
            ScheduledSubStep::Unsaturated { .. } => return Err(PollErr::Unsaturated),
            ScheduledSubStep::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            ScheduledSubStep::Saturated { .. } => return Err(PollErr::Saturated),
        };

        *this = ScheduledSubStep::Saturated { wrapped_transposer };

        Ok(Poll::Ready(()))
    }

    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>> {
        match self {
            ScheduledSubStep::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn take_finished_transposer(
        self: Pin<&mut Self>,
    ) -> Option<SharedPointer<WrappedTransposer<T, P>, P>> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        match core::mem::replace(this, ScheduledSubStep::Unsaturated { time }) {
            ScheduledSubStep::Unsaturated { .. } => None,
            ScheduledSubStep::Saturating { .. } => None,
            ScheduledSubStep::Saturated { wrapped_transposer } => Some(wrapped_transposer),
        }
    }

    fn desaturate(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        *this = ScheduledSubStep::Unsaturated { time };
    }

    fn sort_phase(&self) -> usize {
        SCHEDULED_SUB_STEP_SORT_PHASE
    }
}
