use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::cell::UnsafeCell;
use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::ptr::NonNull;

use archery::{SharedPointer, SharedPointerKind};

use super::interpolate_context::StepInterpolateContext;
use super::wrapped_transposer::WrappedTransposer;
use super::InputState;
use crate::transposer::input_state_requester::InputStateManager;
use crate::transposer::Transposer;

pub trait Interpolation<T: Transposer>: Future<Output = T::OutputState> {
    fn get_input_state(self: Pin<&mut Self>) -> &mut InputStateManager<T>;
}

#[derive(Default)]
enum InterpolationInner<T, Fb: FutureBuilder<T>> {
    Uninit {
        future_builder: Fb,
        input_state: InputStateManager<T>,
    },
    InProgress {
        future: MaybeUninit<Fb::Future>,

        input_state: UnsafeCell<InputStateManager<T>>,

        // future contains a reference to input_state.
        _pin: PhantomPinned,
    },
    #[default]
    Dummy,
}

trait FutureBuilder<T> {
    type Output;
    type Future: Future<Output = Self::Output>;
    fn build_future(self, input_state_ptr: NonNull<UnsafeCell<InputStateManager<T>>>) -> Self::Future;
}

impl<Fn, T, Fut> FutureBuilder<T> for Fn
where
    Fn: FnOnce(NonNull<UnsafeCell<InputStateManager<T>>>) -> Fut,
    Fut: Future,
{
    type Output = Fut::Output;
    type Future = Fut;

    fn build_future(self, input_state_ptr: NonNull<UnsafeCell<InputStateManager<T>>>) -> Self::Future {
        self(input_state_ptr)
    }
}

pub(crate) fn new_interpolation<T: Transposer, P: SharedPointerKind>(
    interpolation_time: T::Time,
    wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
) -> impl Interpolation<T> {
    let input_state = NonNull::from(Box::leak(Box::new(((), InputStateManager::default()))));

    let future_builder =
        move |input_state| interpolate(interpolation_time, wrapped_transposer, input_state);

    InterpolationInner::Uninit {
        input_state: InputStateManager::default(),
        future_builder,
    }
}

async fn interpolate<T: Transposer, P: SharedPointerKind>(
    interpolation_time: T::Time,
    wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    // mutable references must not be held over await points.
    input_state: NonNull<((), InputStateManager<T>)>,
) -> T::OutputState {
    let borrowed = wrapped_transposer.as_ref();
    let transposer = &borrowed.transposer;
    let metadata = &borrowed.metadata;

    let mut context = StepInterpolateContext::new(interpolation_time, metadata, input_state);

    transposer.interpolate(&mut context).await
}

impl<O, Is: Default, Fb: FutureBuilder<Is, Output = O>> Future for InterpolationInner<Is, Fb> {
    type Output = O;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { self.get_unchecked_mut() };

        if let InterpolationInner::Uninit {
            future_builder,
            input_state,
        } = core::mem::take(unpinned)
        {
            *unpinned = InterpolationInner::InProgress {
                future: MaybeUninit::uninit(),
                input_state: UnsafeCell::new(input_state),
                _pin: PhantomPinned,
            };

            let input_state_pointer = NonNull::from(match &unpinned {
                InterpolationInner::InProgress { input_state, .. } => input_state,
                _ => unreachable!(),
            });

            match unpinned {
                InterpolationInner::InProgress { future, .. } => {
                    future.write(future_builder.build_future(input_state_pointer));
                }
                _ => unreachable!(),
            }
        };

        match unpinned {
            InterpolationInner::InProgress { future, .. } => {
                unsafe { Pin::new_unchecked(future.assume_init_mut()) }.poll(cx)
            }
            _ => unreachable!(),
        }
    }
}

impl<Out, Is: Default, Fb: FutureBuilder<Is, Output = Out>> Interpolation<Is, Out>
    for InterpolationInner<Is, Fb>
{
    fn get_input_state(self: Pin<&mut Self>) -> &mut Is {
        let unpinned = unsafe { self.get_unchecked_mut() };

        match unpinned {
            InterpolationInner::Uninit { input_state, .. } => input_state,
            InterpolationInner::InProgress { input_state, .. } => {
                // SAFETY: this is only bad if we never hold mutable references to this over await points
                // inside the future, which we must be careful not to do.
                unsafe { &mut *input_state.get() }
            }
            InterpolationInner::Dummy => unreachable!(),
        }
    }
}
