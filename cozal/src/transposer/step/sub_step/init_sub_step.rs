use std::{
    future::Future,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    input_state_manager::InputStateManager,
    step::{wrapped_transposer::WrappedTransposer, OutputEventManager, PollErr},
    Transposer,
};

use super::{BoxedSubStep, StartSaturateErr, SubStep};

#[allow(dead_code)]
pub fn new_init_sub_step<T: Transposer, P: SharedPointerKind>(
    transposer: T,
    rng_seed: [u8; 32],
    start_time: T::Time,
    shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
) -> impl SubStep<T, P> {
    new_init_sub_step_internal(transposer, rng_seed, start_time, shared_step_state)
}

pub fn new_init_boxed_sub_step<'a, T: Transposer + 'a, P: SharedPointerKind + 'a>(
    transposer: T,
    rng_seed: [u8; 32],
    start_time: T::Time,
    shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
) -> BoxedSubStep<'a, T, P> {
    BoxedSubStep::new(Box::new(new_init_sub_step(
        transposer,
        rng_seed,
        start_time,
        shared_step_state,
    )))
}

#[allow(unused)]
enum InitSubStepStatus<T: Transposer, P: SharedPointerKind> {
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

impl<T: Transposer, P: SharedPointerKind> SubStep<T, P> for InitSubStepStatus<T, P> {
    fn is_init(&self) -> bool {
        true
    }

    fn is_unsaturated(&self) -> bool {
        false
    }

    fn is_saturating(&self) -> bool {
        matches!(self, InitSubStepStatus::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, InitSubStepStatus::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match self {
            InitSubStepStatus::Unsaturated { time } => *time,
            InitSubStepStatus::Saturating { time, .. } => *time,
            InitSubStepStatus::Saturated { wrapped_transposer } => {
                wrapped_transposer.metadata.last_updated.time
            }
        }
    }

    fn dyn_cmp(&self, other: &dyn SubStep<T, P>) -> std::cmp::Ordering {
        match other.is_init() {
            true => std::cmp::Ordering::Equal,
            false => std::cmp::Ordering::Less,
        }
    }

    fn start_saturate(
        self: Pin<&mut Self>,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> Result<(), StartSaturateErr> {
        let _ = wrapped_transposer;
        let _ = shared_step_state;

        match &*self {
            InitSubStepStatus::Unsaturated { .. } => Err(StartSaturateErr::SubStepTimeIsPast),
            _ => Err(StartSaturateErr::NotUnsaturated),
        }
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match this {
            InitSubStepStatus::Unsaturated { .. } => return Err(PollErr::Unsaturated),
            InitSubStepStatus::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            InitSubStepStatus::Saturated { .. } => return Err(PollErr::Saturated),
        };

        *this = InitSubStepStatus::Saturated { wrapped_transposer };

        Ok(Poll::Ready(()))
    }

    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>> {
        match self {
            InitSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn take_finished_transposer(
        self: Pin<&mut Self>,
    ) -> Option<SharedPointer<WrappedTransposer<T, P>, P>> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        match core::mem::replace(this, InitSubStepStatus::Unsaturated { time }) {
            InitSubStepStatus::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn desaturate(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        *this = InitSubStepStatus::Unsaturated { time };
    }
}

fn new_init_sub_step_internal<T: Transposer, P: SharedPointerKind>(
    transposer: T,
    rng_seed: [u8; 32],
    start_time: T::Time,
    shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
) -> InitSubStepStatus<T, P> {
    InitSubStepStatus::Saturating {
        time: start_time,
        future: wrapped_handler::handle(transposer, rng_seed, start_time, shared_step_state),
    }
}

mod wrapped_handler {
    use super::*;

    pub type WrappedHandlerFuture<T: Transposer, P: SharedPointerKind> =
        impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>;

    pub fn handle<T: Transposer, P: SharedPointerKind>(
        transposer: T,
        rng_seed: [u8; 32],
        start_time: T::Time,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> WrappedHandlerFuture<T, P> {
        async move { WrappedTransposer::init(transposer, rng_seed, start_time, shared_step_state).await }
    }
}
