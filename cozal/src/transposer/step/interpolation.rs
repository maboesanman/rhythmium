use std::{
    future::Future,
    marker::PhantomPinned,
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{input_state_manager::InputStateManager, Transposer};

use super::{interpolate_context::StepInterpolateContext, wrapped_transposer::WrappedTransposer};

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
    InProgress {
        interpolation_time: T::Time,
        future: MaybeUninit<interpolate::FutureImpl<T, P>>,
        input_state: InputStateManager<T>,

        // future contains a reference to input_state.
        _pin: PhantomPinned,
    },
    #[default]
    Dummy,
}

impl<T: Transposer, P: SharedPointerKind> Interpolation<T, P> {
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

    pub fn get_time(&self) -> T::Time {
        match &self.inner {
            InterpolationInner::Uninit {
                interpolation_time, ..
            }
            | InterpolationInner::InProgress {
                interpolation_time, ..
            } => *interpolation_time,
            InterpolationInner::Dummy => unreachable!(),
        }
    }

    pub fn get_input_state_manager(self: Pin<&mut Self>) -> &mut InputStateManager<T> {
        let unpinned = unsafe { self.get_unchecked_mut() };

        match &mut unpinned.inner {
            InterpolationInner::Uninit { input_state, .. }
            | InterpolationInner::InProgress { input_state, .. } => input_state,
            InterpolationInner::Dummy => unreachable!(),
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> Future for Interpolation<T, P> {
    type Output = T::OutputState;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { self.get_unchecked_mut() };

        let future = match &mut unpinned.inner {
            InterpolationInner::InProgress { future, .. } => future,
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

                unpinned.inner = InterpolationInner::InProgress {
                    interpolation_time,
                    future: MaybeUninit::uninit(),
                    input_state,
                    _pin: PhantomPinned,
                };

                let input_state = match &mut unpinned.inner {
                    InterpolationInner::InProgress { input_state, .. } => input_state,
                    _ => unreachable!(),
                };

                let input_state = NonNull::from(input_state);

                let future = match &mut unpinned.inner {
                    InterpolationInner::InProgress { future, .. } => future,
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

            transposer.interpolate(&mut context).await
        }
    }
}
