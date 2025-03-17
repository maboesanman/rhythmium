use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use smallvec::{SmallVec, smallvec};

use crate::transposer::Transposer;

pub struct OutputEventManager<T: Transposer> {
    outputs_to_swallow: usize,
    inner: OutputEventManagerInner<T>,
}

impl<T: Transposer> Default for OutputEventManager<T> {
    fn default() -> Self {
        Self {
            outputs_to_swallow: Default::default(),
            inner: Default::default(),
        }
    }
}

impl<T: Transposer> OutputEventManager<T> {
    pub fn new_with_swallow_count(swallow_count: usize) -> Self {
        Self {
            outputs_to_swallow: swallow_count,
            inner: Default::default(),
        }
    }
    pub fn try_take_value(&mut self) -> Option<T::OutputEvent> {
        match core::mem::take(&mut self.inner) {
            OutputEventManagerInner::Occupied { event, waiting } => {
                for waker in waiting.into_iter() {
                    waker.wake();
                }
                Some(event)
            }
            OutputEventManagerInner::Vacant => None,
        }
    }
}

pub enum OutputEventManagerInner<T: Transposer> {
    // wakers to call when the output slot is vacated.
    Occupied {
        event: T::OutputEvent,
        waiting: SmallVec<[Waker; 1]>,
    },
    Vacant,
}

impl<T: Transposer> Default for OutputEventManagerInner<T> {
    fn default() -> Self {
        Self::Vacant
    }
}

#[derive(Default)]
enum EmitOutputFutureInner<T: Transposer> {
    WaitingForVacate {
        value_to_emit: T::OutputEvent,
    },
    #[default]
    Emitted,
}
pub struct EmitOutputFuture<'a, T: Transposer> {
    inner: EmitOutputFutureInner<T>,
    manager: NonNull<OutputEventManager<T>>,
    phantom: PhantomData<&'a mut OutputEventManager<T>>,
}

impl<T: Transposer> EmitOutputFuture<'_, T> {
    pub fn new(mut manager: NonNull<OutputEventManager<T>>, value_to_emit: T::OutputEvent) -> Self {
        let manager_mut = unsafe { manager.as_mut() };
        if manager_mut.outputs_to_swallow > 0 {
            manager_mut.outputs_to_swallow -= 1;
            return Self {
                inner: EmitOutputFutureInner::Emitted,
                manager,
                phantom: PhantomData,
            };
        }
        Self {
            inner: EmitOutputFutureInner::WaitingForVacate { value_to_emit },
            manager,
            phantom: PhantomData,
        }
    }
}

impl<T: Transposer> Future for EmitOutputFuture<'_, T> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let manager = unsafe { this.manager.as_mut() };
        let fut_inner = core::mem::take(&mut this.inner);
        let manager_inner = &mut manager.inner;

        match (fut_inner, manager_inner) {
            (
                EmitOutputFutureInner::WaitingForVacate { value_to_emit },
                OutputEventManagerInner::Occupied { waiting, .. },
            ) => {
                waiting.push(cx.waker().clone());
                this.inner = EmitOutputFutureInner::WaitingForVacate { value_to_emit };
                Poll::Pending
            }
            (
                EmitOutputFutureInner::WaitingForVacate { value_to_emit },
                OutputEventManagerInner::Vacant,
            ) => {
                manager.inner = OutputEventManagerInner::Occupied {
                    event: value_to_emit,
                    waiting: smallvec![cx.waker().clone()],
                };
                Poll::Pending
            }
            (EmitOutputFutureInner::Emitted, _) => Poll::Ready(()),
        }
    }
}
