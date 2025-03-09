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

use super::{StartSaturateErr, SubStep};

pub enum InitSubStep<T: Transposer + Clone, P: SharedPointerKind> {
    Saturating {
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
    ) -> WrappedHandlerFuture<T, P> {
        async move {
            let transposer_mut = SharedPointer::make_mut(&mut wrapped_transposer);
            transposer_mut.init().await;
            wrapped_transposer
        }
    }
}

impl<T: Transposer + Clone, P: SharedPointerKind> InitSubStep<T, P> {
    pub fn new(
        transposer: T,
        rng_seed: [u8; 32],
    ) -> Self {
        let wrapped_transposer = WrappedTransposer::new(transposer, rng_seed);
        InitSubStep::Saturating {
            future: wrapped_handler::handle(wrapped_transposer),
        }
    }

    fn is_saturating(&self) -> bool {
        matches!(self, InitSubStep::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, InitSubStep::Saturated { .. })
    }

    pub fn poll(self: Pin<&mut Self>, waker: &Waker) -> Result<Poll<()>, super::PollErr> {
        let this = unsafe { self.get_unchecked_mut() };

        let wrapped_transposer = match this {
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
}
