use std::{
    any::TypeId,
    cmp::Ordering,
    future::Future,
    marker::PhantomPinned,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use super::{BoxedSubStep, StartSaturateErr, SubStep};
use crate::transposer::{
    input_state_manager::InputStateManager,
    step::{wrapped_transposer::WrappedTransposer, OutputEventManager, PollErr},
    Transposer, TransposerInput, TransposerInputEventHandler,
};

#[allow(dead_code)]
pub fn new_input_sub_step<T, P, I>(
    time: T::Time,
    input: I,
    input_event: I::InputEvent,
) -> impl SubStep<T, P>
where
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    InputSubStep {
        status: InputSubStepStatus::Unsaturated { time },
        data: InputSubStepData { input, input_event },
        _pinned: PhantomPinned,
    }
}

#[allow(dead_code)]
pub fn new_input_boxed_sub_step<'a, T, P, I>(
    time: T::Time,
    input: I,
    input_event: I::InputEvent,
) -> BoxedSubStep<'a, T, P>
where
    P: SharedPointerKind + 'a,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone + 'a,
{
    BoxedSubStep::new(Box::new(new_input_sub_step::<T, P, I>(
        time,
        input,
        input_event,
    )))
}

#[allow(unused)]
struct InputSubStepData<T: Transposer, I: TransposerInput<Base = T>> {
    input: I,
    input_event: I::InputEvent,
}

#[allow(unused)]
enum InputSubStepStatus<T: Transposer, P: SharedPointerKind, I: TransposerInput<Base = T>>
where
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    Unsaturated {
        time: T::Time,
    },
    Saturating {
        time: T::Time,
        future: wrapped_handler::WrappedHandlerFuture<T, P, I>,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    },
}

#[allow(unused)]
struct InputSubStep<T: Transposer, P: SharedPointerKind, I: TransposerInput<Base = T>>
where
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    status: InputSubStepStatus<T, P, I>,
    data: InputSubStepData<T, I>,
    _pinned: PhantomPinned,
}

impl<T, P, I> SubStep<T, P> for InputSubStep<T, P, I>
where
    T: Transposer + TransposerInputEventHandler<I> + Clone,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
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

    fn dyn_cmp(&self, other: &dyn SubStep<T, P>) -> Ordering {
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
            Ordering::Equal => {
                // we compared both the input sort and the input type_id in this comparison, so
                // now we can be sure the type of other is Self.
            }
            ne => return ne,
        }

        let other_ptr = other as *const dyn SubStep<T, P> as *const Self;
        let other = unsafe { &*other_ptr };

        match self.data.input.cmp(&other.data.input) {
            Ordering::Equal => {}
            ne => return ne,
        }

        match self.data.input_event.cmp(&other.data.input_event) {
            Ordering::Equal => {}
            ne => return ne,
        }

        // could sort by byte representation of input_event, but not sure about
        // endianness between platforms.
        Ordering::Equal
    }

    fn start_saturate(
        self: Pin<&mut Self>,
        transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
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

        let future = wrapped_handler::handle::<T, P, I>(
            transposer,
            time,
            input,
            input_event,
            shared_step_state,
        );

        this.status = InputSubStepStatus::Saturating { time, future };

        Ok(())
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match &mut this.status {
            InputSubStepStatus::Unsaturated { .. } => return Err(PollErr::Unsaturated),
            InputSubStepStatus::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            InputSubStepStatus::Saturated { .. } => return Err(PollErr::Saturated),
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

        let time = <Self as SubStep<T, P>>::get_time(this);

        match core::mem::replace(&mut this.status, InputSubStepStatus::Unsaturated { time }) {
            InputSubStepStatus::Unsaturated { .. } => None,
            InputSubStepStatus::Saturating { .. } => None,
            InputSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
        }
    }

    fn desaturate(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        this.status = InputSubStepStatus::Unsaturated { time };
    }
}

mod wrapped_handler {
    use super::*;

    pub type WrappedHandlerFuture<T, P, I>
    where
        P: SharedPointerKind,
        I: TransposerInput<Base = T>,
        T: Transposer + TransposerInputEventHandler<I> + Clone,
    = impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>;

    pub fn handle<T, P, I>(
        mut wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        time: T::Time,
        input: NonNull<I>,
        input_event: NonNull<I::InputEvent>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> WrappedHandlerFuture<T, P, I>
    where
        P: SharedPointerKind,
        I: TransposerInput<Base = T>,
        T: Transposer + TransposerInputEventHandler<I> + Clone,
    {
        async move {
            let transposer_mut = SharedPointer::make_mut(&mut wrapped_transposer);
            let input = unsafe { input.as_ref() };
            let input_event = unsafe { input_event.as_ref() };
            transposer_mut
                .handle_input(time, input, input_event, shared_step_state)
                .await;
            wrapped_transposer
        }
    }
}
