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

use super::{BoxedSubStep, INPUT_SUB_STEP_SORT_PHASE, StartSaturateErr, SubStep};
use crate::transposer::{
    Transposer, TransposerInput, TransposerInputEventHandler,
    input_erasure::{HasErasedInputExt, HasInput},
    input_state_manager::InputStateManager,
    output_event_manager::OutputEventManager,
    step::{step::PollErr, wrapped_transposer::WrappedTransposer},
};

pub struct InputSubStep<T: Transposer, P: SharedPointerKind, I: TransposerInput<Base = T>>
where
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    T: Transposer + TransposerInputEventHandler<I> + Clone,
{
    status: InputSubStepStatus<T, P, I>,
    data: InputSubStepData<T, I>,
    _pinned: PhantomPinned,
}

struct InputSubStepData<T: Transposer, I: TransposerInput<Base = T>> {
    input: I,
    input_event: I::InputEvent,
}

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

mod wrapped_handler {
    use crate::transposer::output_event_manager::OutputEventManager;

    use super::*;

    pub type WrappedHandlerFuture<T, P, I>
    where
        P: SharedPointerKind,
        I: TransposerInput<Base = T>,
        T: Transposer + TransposerInputEventHandler<I> + Clone,
    = impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>;

    #[define_opaque(WrappedHandlerFuture)]
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

impl<T, P, I> InputSubStep<T, P, I>
where
    T: Transposer + TransposerInputEventHandler<I> + Clone,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
{
    pub fn new(time: T::Time, input: I, input_event: I::InputEvent) -> Self {
        Self {
            status: InputSubStepStatus::Unsaturated { time },
            data: InputSubStepData { input, input_event },
            _pinned: PhantomPinned,
        }
    }

    pub fn new_boxed<'a>(
        time: T::Time,
        input: I,
        input_event: I::InputEvent,
    ) -> BoxedSubStep<'a, T, P>
    where
        T: 'a,
        P: 'a,
    {
        BoxedSubStep::new(Box::new(Self::new(time, input, input_event)))
    }

    pub fn get_input(&self) -> &I {
        &self.data.input
    }
}

impl<T, P, I> HasInput<T> for InputSubStep<T, P, I>
where
    T: Transposer + TransposerInputEventHandler<I> + Clone,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
{
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        &self.data.input
    }
}

unsafe impl<T, P, I> SubStep<T, P> for InputSubStep<T, P, I>
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

    fn input_hash(&self) -> Option<u64> {
        Some(self.get_hash())
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
                wrapped_transposer.metadata.last_updated.unwrap().time
            }
        }
    }

    fn dyn_cmp(&self, other: &dyn SubStep<T, P>) -> Ordering {
        match self.get_time().cmp(&other.get_time()) {
            Ordering::Equal => {}
            ne => return ne,
        };

        match self.sort_phase().cmp(&other.sort_phase()) {
            Ordering::Equal => {}
            ne => return ne,
        }

        // self and other are both inputs

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

        self.data.input_event.cmp(&other.data.input_event)
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

        if let Some(t) = transposer.metadata.last_updated
            && t.time > time
        {
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

    fn sort_phase(&self) -> usize {
        INPUT_SUB_STEP_SORT_PHASE
    }
}
