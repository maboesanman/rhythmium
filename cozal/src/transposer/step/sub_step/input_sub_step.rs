use std::{
    any::TypeId,
    cell::UnsafeCell,
    cmp::Ordering,
    future::Future,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    step::{wrapped_transposer::WrappedTransposer, InputState},
    Transposer, TransposerInput, TransposerInputEventHandler,
};

use super::{PollErr, StartSaturateErr, SubStep};

#[allow(dead_code)]
pub fn new_input_sub_step<T, P, I, S>(
    time: T::Time,
    input: I,
    input_event: I::InputEvent,
) -> impl SubStep<T, P, S>
where
    S: InputState<T>,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    new_input_sub_step_internal::<T, P, I, S>(time, input, input_event)
}

struct InputSubStepData<T: Transposer, I: TransposerInput<Base = T>> {
    input: I,
    input_event: I::InputEvent,
}

enum InputSubStepStatus<T: Transposer, P: SharedPointerKind, S, Fut> {
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

struct InputSubStep<T: Transposer, P: SharedPointerKind, I: TransposerInput<Base = T>, S, Fut> {
    status: InputSubStepStatus<T, P, S, Fut>,
    data: InputSubStepData<T, I>,
    _pinned: PhantomPinned,
}

impl<T, P, I, S, Fut> SubStep<T, P, S> for InputSubStep<T, P, I, S, Fut>
where
    T: Transposer + TransposerInputEventHandler<I> + Clone,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    S: InputState<T>,
    Fut: Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>,
{
    fn is_input(&self) -> bool {
        true
    }

    fn input_sort(&self) -> Option<(u64, TypeId)> {
        Some((I::SORT, TypeId::of::<I>()))
    }

    fn is_unsaturated(&self) -> bool {
        matches!(self.status, InputSubStepStatus::Unsaturated { .. })
    }

    fn is_saturating(&self) -> bool {
        matches!(self.status, InputSubStepStatus::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self.status, InputSubStepStatus::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match &self.status {
            InputSubStepStatus::Unsaturated { time, .. } => *time,
            InputSubStepStatus::Saturating { time, .. } => *time,
            InputSubStepStatus::Saturated { wrapped_transposer } => {
                wrapped_transposer.metadata.last_updated.time
            }
        }
    }

    fn cmp(&self, other: &dyn SubStep<T, P, S>) -> Ordering {
        match self.get_time().cmp(&other.get_time()) {
            Ordering::Equal => {}
            ne => return ne,
        };

        if other.is_init() {
            return Ordering::Greater;
        }

        if other.is_scheduled() {
            return Ordering::Less;
        }

        match self.input_sort().cmp(&other.input_sort()) {
            Ordering::Equal => {}
            ne => return ne,
        }

        let other_addr = (other as *const dyn SubStep<T, P, S>).addr();
        let other_ptr = (self as *const Self).with_addr(other_addr);
        let other = unsafe { &*other_ptr };

        match self.data.input.cmp(&other.data.input) {
            Ordering::Equal => {}
            ne => return ne,
        }

        self.data.input_event.cmp(&other.data.input_event)
    }

    fn start_saturate(
        self: Pin<&mut Self>,
        transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<UnsafeCell<S>>,
        outputs_to_swallow: usize,
    ) -> Result<(), StartSaturateErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = match &this.status {
            InputSubStepStatus::Unsaturated { time, .. } => *time,
            _ => return Err(StartSaturateErr::NotUnsaturated),
        };

        if transposer.metadata.last_updated.time > time {
            return Err(StartSaturateErr::SubStepTimeIsPast);
        }

        let input = NonNull::from(&this.data.input);
        let input_event = NonNull::from(&this.data.input_event);

        let future = shared_pointer_update::<T, P, I, S>(
            transposer,
            time,
            input,
            input_event,
            shared_step_state,
            outputs_to_swallow,
        );

        // // debug_assert_eq!(TypeId::of::<Fut>(), future.type_id());

        // Safety: this future type is only ever created by invoking the `shared_pointer_update` function,
        // so the future returned by it is exactly `Fut`.
        let corrected_future = unsafe { core::mem::transmute_copy::<_, Fut>(&future) };
        core::mem::forget(future);

        this.status = InputSubStepStatus::Saturating {
            time,
            future: corrected_future,
        };

        Ok(())
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match &mut this.status {
            InputSubStepStatus::Unsaturated { .. } => return Err(PollErr::NotSaturating),
            InputSubStepStatus::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            InputSubStepStatus::Saturated { .. } => return Err(PollErr::NotSaturating),
        };

        this.status = InputSubStepStatus::Saturated { wrapped_transposer };

        Ok(Poll::Ready(()))
    }

    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>> {
        match self.status {
            InputSubStepStatus::Saturated {
                ref wrapped_transposer,
            } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn take_finished_transposer(
        self: std::pin::Pin<&mut Self>,
    ) -> Option<SharedPointer<WrappedTransposer<T, P>, P>> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P, S>>::get_time(this);

        match core::mem::replace(
            &mut this.status,
            InputSubStepStatus::Unsaturated {
                time,
                phantom: PhantomData,
            },
        ) {
            InputSubStepStatus::Unsaturated { .. } => None,
            InputSubStepStatus::Saturating { .. } => None,
            InputSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
        }
    }
}

async fn shared_pointer_update<T, P, I, S>(
    mut wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    time: T::Time,
    input: NonNull<I>,
    input_event: NonNull<I::InputEvent>,
    shared_step_state: NonNull<UnsafeCell<S>>,
    outputs_to_swallow: usize,
) -> SharedPointer<WrappedTransposer<T, P>, P>
where
    S: InputState<T>,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    let transposer_mut = SharedPointer::make_mut(&mut wrapped_transposer);
    let input = unsafe { input.as_ref() };
    let input_event = unsafe { input_event.as_ref() };
    transposer_mut
        .handle_input(
            time,
            input,
            input_event,
            shared_step_state,
            outputs_to_swallow,
        )
        .await;
    wrapped_transposer
}

fn new_input_sub_step_internal<T, P, I, S>(
    time: T::Time,
    input: I,
    input_event: I::InputEvent,
) -> InputSubStep<T, P, I, S, impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>>
where
    S: InputState<T>,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    // This is a trick to get the compiler to understand the type of the future.
    #[allow(unreachable_code)]
    if false {
        return InputSubStep {
            status: InputSubStepStatus::Saturating {
                time: unreachable!(),
                future: shared_pointer_update::<T, P, I, S>(
                    unreachable!(),
                    unreachable!(),
                    unreachable!(),
                    unreachable!(),
                    unreachable!(),
                    unreachable!(),
                ),
            },
            data: unreachable!(),
            _pinned: unreachable!(),
        };
    }

    InputSubStep {
        status: InputSubStepStatus::Unsaturated {
            time,
            phantom: PhantomData,
        },
        data: InputSubStepData { input, input_event },
        _pinned: PhantomPinned,
    }
}
