use std::{
    cell::UnsafeCell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    input_state_requester::InputStateManager, step::{wrapped_transposer::WrappedTransposer, InputState, OutputState}, Transposer
};

use super::{PollErr, StartSaturateErr, SubStep};

#[allow(dead_code)]
pub fn new_scheduled_sub_step<T: Transposer + Clone, P: SharedPointerKind, S: InputState<T> + OutputState<T>>(
    time: T::Time,
) -> impl SubStep<T, P, S> {
    new_scheduled_sub_step_internal::<T, P, S>(time)
}

enum ScheduledSubStepStatus<T: Transposer, P: SharedPointerKind, S, Fut> {
    Unsaturated {
        time: T::Time,
        phantom: PhantomData<fn(S)>,
    },
    Saturating {
        time: T::Time,
        future: Fut,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    },
}

impl<T, P, S, Fut> SubStep<T, P, S> for ScheduledSubStepStatus<T, P, S, Fut>
where
    T: Transposer + Clone,
    P: SharedPointerKind,
    S: InputState<T> + OutputState<T>,
    Fut: Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>,
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

    fn cmp(&self, other: &dyn SubStep<T, P, S>) -> std::cmp::Ordering {
        match other.is_scheduled() {
            true => std::cmp::Ordering::Equal,
            false => std::cmp::Ordering::Greater,
        }
    }

    fn start_saturate(
        self: Pin<&mut Self>,
        transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(S, InputStateManager<T>)>,
    ) -> Result<(), StartSaturateErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = match this {
            ScheduledSubStepStatus::Unsaturated { time, .. } => *time,
            _ => return Err(StartSaturateErr::NotUnsaturated),
        };

        if transposer.metadata.last_updated.time > time {
            return Err(StartSaturateErr::SubStepTimeIsPast);
        }

        let future = shared_pointer_update::<T, P, S>(
            transposer,
            time,
            shared_step_state,
        );

        // debug_assert_eq!(TypeId::of::<Fut>(), future.type_id());

        // Safety: this future type is only ever created by invoking the `shared_pointer_update` function,
        // so the future returned by it is exactly `Fut`.
        let corrected_future = unsafe { core::mem::transmute_copy::<_, Fut>(&future) };
        core::mem::forget(future);

        *this = ScheduledSubStepStatus::Saturating {
            time,
            future: corrected_future,
        };

        Ok(())
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match this {
            ScheduledSubStepStatus::Unsaturated { .. } => return Err(PollErr::NotSaturating),
            ScheduledSubStepStatus::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            ScheduledSubStepStatus::Saturated { .. } => return Err(PollErr::NotSaturating),
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

        let time = <Self as SubStep<T, P, S>>::get_time(this);

        match core::mem::replace(
            this,
            ScheduledSubStepStatus::Unsaturated {
                time,
                phantom: PhantomData,
            },
        ) {
            ScheduledSubStepStatus::Unsaturated { .. } => None,
            ScheduledSubStepStatus::Saturating { .. } => None,
            ScheduledSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
        }
    }
}

async fn shared_pointer_update<T: Transposer + Clone, P: SharedPointerKind, S: InputState<T> + OutputState<T>>(
    mut wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    time: T::Time,
    shared_step_state: NonNull<(S, InputStateManager<T>)>,
) -> SharedPointer<WrappedTransposer<T, P>, P> {
    let transposer_mut = SharedPointer::make_mut(&mut wrapped_transposer);
    transposer_mut
        .handle_scheduled(time, shared_step_state)
        .await;
    wrapped_transposer
}

fn new_scheduled_sub_step_internal<
    T: Transposer + Clone,
    P: SharedPointerKind,
    S: InputState<T> + OutputState<T>,
>(
    time: T::Time,
) -> ScheduledSubStepStatus<T, P, S, impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>>
{
    // This is a trick to get the compiler to understand the type of the future.
    #[allow(unreachable_code)]
    if false {
        return ScheduledSubStepStatus::Saturating {
            time,
            future: shared_pointer_update::<T, P, S>(
                unreachable!(),
                unreachable!(),
                unreachable!(),
            ),
        };
    }

    ScheduledSubStepStatus::Unsaturated {
        time,
        phantom: PhantomData,
    }
}
