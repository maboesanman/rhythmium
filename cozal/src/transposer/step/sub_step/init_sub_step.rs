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

use super::{BoxedSubStep, StartSaturateErr, SubStep, INIT_SUB_STEP_SORT_PHASE};

pub enum InitSubStep<T: Transposer + Clone, P: SharedPointerKind> {
    ForeverUnsaturated {
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

    pub fn handle<T: Transposer + Clone, P: SharedPointerKind>(
        mut wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> WrappedHandlerFuture<T, P> {
        async move {
            let transposer_mut = SharedPointer::make_mut(&mut wrapped_transposer);
            transposer_mut.init(shared_step_state).await;
            wrapped_transposer
        }
    }
}

impl<T: Transposer + Clone, P: SharedPointerKind> InitSubStep<T, P> {
    pub fn new(
        transposer: T,
        rng_seed: [u8; 32],
        time: T::Time,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> Self {
        let wrapped_transposer = WrappedTransposer::new(transposer, rng_seed, time);
        InitSubStep::Saturating {
            time,
            future: wrapped_handler::handle(wrapped_transposer, shared_step_state),
        }
    }

    pub fn new_boxed<'a>(
        transposer: T,
        rng_seed: [u8; 32],
        time: T::Time,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> BoxedSubStep<'a, T, P>
    where
        T: 'a,
        P: 'a,
    {
        BoxedSubStep::new(Box::new(Self::new(
            transposer,
            rng_seed,
            time,
            shared_step_state,
        )))
    }
}

unsafe impl<T: Transposer + Clone, P: SharedPointerKind> SubStep<T, P> for InitSubStep<T, P> {
    fn is_init(&self) -> bool {
        true
    }

    fn is_unsaturated(&self) -> bool {
        matches!(self, InitSubStep::ForeverUnsaturated { .. })
    }

    fn is_saturating(&self) -> bool {
        matches!(self, InitSubStep::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, InitSubStep::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match self {
            InitSubStep::ForeverUnsaturated { time, .. } => *time,
            InitSubStep::Saturating { time, .. } => *time,
            InitSubStep::Saturated { wrapped_transposer } => {
                wrapped_transposer.metadata.last_updated.time
            }
        }
    }

    fn dyn_cmp(&self, other: &dyn SubStep<T, P>) -> std::cmp::Ordering {
        // there is only one possible init.
        // init and register input both happen before everything else so we don't need to check time.
        self.sort_phase().cmp(&other.sort_phase())
    }

    fn start_saturate(
        self: Pin<&mut Self>,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> Result<(), StartSaturateErr> {
        let _ = wrapped_transposer;
        let _ = shared_step_state;

        match &*self {
            InitSubStep::ForeverUnsaturated { .. } => Err(StartSaturateErr::SubStepTimeIsPast),
            // InitSubStep::Unsaturated { .. } => Err(StartSaturateErr::SubStepTimeIsPast),
            _ => Err(StartSaturateErr::NotUnsaturated),
        }
    }

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match this {
            InitSubStep::ForeverUnsaturated { .. } => return Err(PollErr::Unsaturated),
            InitSubStep::Saturating { future, .. } => {
                let pinned = unsafe { Pin::new_unchecked(future) };

                match pinned.poll(&mut Context::from_waker(waker)) {
                    Poll::Ready(wrapped_transposer) => wrapped_transposer,
                    Poll::Pending => return Ok(Poll::Pending),
                }
            }
            InitSubStep::Saturated { .. } => return Err(PollErr::Saturated),
        };

        *this = InitSubStep::Saturated { wrapped_transposer };

        Ok(Poll::Ready(()))
    }

    fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>> {
        match self {
            InitSubStep::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn take_finished_transposer(
        self: Pin<&mut Self>,
    ) -> Option<SharedPointer<WrappedTransposer<T, P>, P>> {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        match core::mem::replace(this, InitSubStep::ForeverUnsaturated { time }) {
            InitSubStep::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }

    fn desaturate(self: Pin<&mut Self>) {
        let this = unsafe { self.get_unchecked_mut() };

        let time = <Self as SubStep<T, P>>::get_time(this);

        *this = InitSubStep::ForeverUnsaturated { time };
    }

    fn sort_phase(&self) -> usize {
        INIT_SUB_STEP_SORT_PHASE
    }
}
