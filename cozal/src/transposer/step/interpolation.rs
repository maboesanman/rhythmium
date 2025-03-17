use std::{
    future::Future,
    marker::PhantomPinned,
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{Transposer, input_state_manager::InputStateManager};

use super::{interpolate_context::StepInterpolateContext, wrapped_transposer::WrappedTransposer};

/// A future for producing the interpolated output state from a transposer.
///
/// This future is a bit odd since the `get_input_state_manager` method must be called in between
/// polling, to determine if an input state has been requested. For this reason it is unlikely that this
/// future will be awaited directly, but rather it will be used as part of a larger async structure
/// (in particular a cozal source).
pub struct Interpolation<T: Transposer, P: SharedPointerKind> {
    inner: InterpolationInner<T, P>,
}

#[derive(Default)]
enum InterpolationInner<T: Transposer, P: SharedPointerKind> {
    Uninit {
        interpolation_time: T::Time,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        input_state: InputStateManager<T>,
    },
    InProgress(InterpolateInnerInProgress<T, P>),
    #[default]
    Dummy,
}

struct InterpolateInnerInProgress<T: Transposer, P: SharedPointerKind> {
    interpolation_time: T::Time,
    future: MaybeUninit<interpolate::FutureImpl<T, P>>,
    input_state: InputStateManager<T>,

    // future contains a reference to input_state.
    _pin: PhantomPinned,
}

impl<T: Transposer, P: SharedPointerKind> Drop for InterpolateInnerInProgress<T, P> {
    fn drop(&mut self) {
        unsafe { self.future.assume_init_drop() };
    }
}

impl<T: Transposer, P: SharedPointerKind> Interpolation<T, P> {
    /// Create a new interpolation future.
    pub fn new(
        interpolation_time: T::Time,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    ) -> Self {
        Self {
            inner: InterpolationInner::Uninit {
                interpolation_time,
                wrapped_transposer,
                input_state: InputStateManager::default(),
            },
        }
    }

    /// Get the time of the interpolation.
    pub fn get_time(&self) -> T::Time {
        match &self.inner {
            InterpolationInner::Uninit {
                interpolation_time, ..
            }
            | InterpolationInner::InProgress(InterpolateInnerInProgress {
                interpolation_time,
                ..
            }) => *interpolation_time,
            InterpolationInner::Dummy => unreachable!(),
        }
    }

    /// Get the input state manager.
    ///
    /// This method must be called after polling returns `Poll::Pending` to determine if an input state
    /// has been requested. That is why the signature requires a pinned mutable reference.
    pub fn get_input_state_manager(self: Pin<&mut Self>) -> &mut InputStateManager<T> {
        let unpinned = unsafe { self.get_unchecked_mut() };

        match &mut unpinned.inner {
            InterpolationInner::Uninit { input_state, .. }
            | InterpolationInner::InProgress(InterpolateInnerInProgress { input_state, .. }) => {
                input_state
            }
            InterpolationInner::Dummy => unreachable!(),
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> Future for Interpolation<T, P> {
    type Output = T::OutputState;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { self.get_unchecked_mut() };

        let future = match &mut unpinned.inner {
            InterpolationInner::InProgress(InterpolateInnerInProgress { future, .. }) => future,
            InterpolationInner::Uninit { .. } => {
                let (interpolation_time, wrapped_transposer, input_state) =
                    match core::mem::take(&mut unpinned.inner) {
                        InterpolationInner::Uninit {
                            interpolation_time,
                            wrapped_transposer,
                            input_state,
                        } => (interpolation_time, wrapped_transposer, input_state),
                        _ => unreachable!(),
                    };

                unpinned.inner = InterpolationInner::InProgress(InterpolateInnerInProgress {
                    interpolation_time,
                    future: MaybeUninit::uninit(),
                    input_state,
                    _pin: PhantomPinned,
                });

                let input_state = match &mut unpinned.inner {
                    InterpolationInner::InProgress(InterpolateInnerInProgress {
                        input_state,
                        ..
                    }) => input_state,
                    _ => unreachable!(),
                };

                let input_state = NonNull::from(input_state);

                let future = match &mut unpinned.inner {
                    InterpolationInner::InProgress(InterpolateInnerInProgress {
                        future, ..
                    }) => future,
                    _ => unreachable!(),
                };

                *future = MaybeUninit::new(interpolate::invoke(
                    interpolation_time,
                    wrapped_transposer,
                    input_state,
                ));
                future
            }
            InterpolationInner::Dummy => unreachable!(),
        };

        let future = unsafe { future.assume_init_mut() };
        let pin = unsafe { Pin::new_unchecked(future) };

        pin.poll(cx)
    }
}

mod interpolate {
    use crate::transposer::context::InterpolateContext;

    use super::*;

    pub type FutureImpl<T: Transposer, P: SharedPointerKind> = impl Future<Output = T::OutputState>;

    pub fn invoke<T: Transposer, P: SharedPointerKind>(
        interpolation_time: T::Time,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
        // mutable references must not be held over await points.
        input_state: NonNull<InputStateManager<T>>,
    ) -> FutureImpl<T, P> {
        async move {
            let borrowed = wrapped_transposer.as_ref();
            let transposer = &borrowed.transposer;
            let metadata = &borrowed.metadata;

            let mut context =
                StepInterpolateContext::new(interpolation_time, metadata, input_state);

            transposer
                .interpolate(InterpolateContext::new_mut(&mut context))
                .await
        }
    }
}
