use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{
    Transposer,
    step::{step::PollErr, wrapped_transposer::WrappedTransposer},
};

pub enum InitSubStep<T: Transposer, P: SharedPointerKind> {
    Saturating {
        future: wrapped_handler::WrappedHandlerFuture<T, P>,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    },
}

impl<T: Transposer, P: SharedPointerKind> Debug for InitSubStep<T, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saturating { .. } => f.debug_struct("Saturating").finish(),
            Self::Saturated { .. } => f.debug_struct("Saturated").finish(),
        }
    }
}

mod wrapped_handler {
    use super::*;

    pub type WrappedHandlerFuture<T: Transposer, P: SharedPointerKind> =
        impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>;

    #[define_opaque(WrappedHandlerFuture)]
    pub fn handle<T: Transposer, P: SharedPointerKind>(
        mut wrapped_transposer: WrappedTransposer<T, P>,
    ) -> WrappedHandlerFuture<T, P> {
        async move {
            wrapped_transposer.init().await;
            SharedPointer::new(wrapped_transposer)
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> InitSubStep<T, P> {
    pub fn new(transposer: T, rng_seed: [u8; 32]) -> Self {
        let wrapped_transposer = WrappedTransposer::new(transposer, rng_seed);
        InitSubStep::Saturating {
            future: wrapped_handler::handle(wrapped_transposer),
        }
    }

    pub fn is_saturating(&self) -> bool {
        matches!(self, InitSubStep::Saturating { .. })
    }

    pub fn is_saturated(&self) -> bool {
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

    pub fn get_finished_transposer(&self) -> Option<&SharedPointer<WrappedTransposer<T, P>, P>> {
        match self {
            InitSubStep::Saturated { wrapped_transposer } => Some(wrapped_transposer),
            _ => None,
        }
    }
}
