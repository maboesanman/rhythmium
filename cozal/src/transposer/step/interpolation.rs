use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use archery::{ArcTK, SharedPointer, SharedPointerKind};

use super::interpolate_context::StepInterpolateContext;
use super::wrapped_transposer::WrappedTransposer;
use super::InputState;
use crate::transposer::Transposer;

pub struct Interpolation<T: Transposer, Is: InputState<T>, P: SharedPointerKind = ArcTK> {
    future: Pin<Box<dyn Future<Output = T::OutputState>>>,
    input_state: SharedPointer<Is, P>,
}

async fn create_fut<T: Transposer, P: SharedPointerKind, Is: InputState<T>>(
    interpolation_time: T::Time,
    wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    input_state: SharedPointer<Is, P>,
) -> T::OutputState {
    let borrowed = wrapped_transposer.as_ref();
    let transposer = &borrowed.transposer;
    let metadata = &borrowed.metadata;

    let input_state_manager = input_state.get_provider();

    let mut context =
        StepInterpolateContext::new(interpolation_time, metadata, input_state_manager);

    transposer.interpolate(&mut context).await
}

impl<T: Transposer, Is: InputState<T>, P: SharedPointerKind> Interpolation<T, Is, P> {
    pub(crate) fn new(
        interpolation_time: T::Time,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    ) -> Self {
        let input_state = SharedPointer::new(Is::new());

        let future = create_fut(interpolation_time, wrapped_transposer, input_state.clone());

        let future: Pin<Box<dyn '_ + Future<Output = T::OutputState>>> = Box::pin(future);
        let future: Pin<Box<dyn 'static + Future<Output = T::OutputState>>> =
            unsafe { core::mem::transmute(future) };

        Self {
            future,
            input_state,
        }
    }

    pub fn get_input_state(&self) -> &Is {
        &self.input_state
    }
}

impl<T: Transposer, Is: InputState<T>, P: SharedPointerKind> Future for Interpolation<T, Is, P> {
    type Output = T::OutputState;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx)
    }
}
