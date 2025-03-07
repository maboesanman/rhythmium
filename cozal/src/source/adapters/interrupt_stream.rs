use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use futures::Stream;

use crate::source::SourcePoll;

use super::super::source_poll::Interrupt;
use super::super::Source;

pub struct InterruptStream<Src: Source<Time = Instant>, Fut: Future<Output = ()>> {
    source: Box<Src>,
    current_wait: Option<Pin<Box<Fut>>>,
    wait_fn: fn(Instant) -> Fut,
}

impl<Src: Source<Time = Instant>, Fut: Future<Output = ()>> InterruptStream<Src, Fut> {
    pub fn new(mut source: Src, wait_fn: fn(Instant) -> Fut) -> Self {
        source.advance_final();
        Self {
            source: Box::new(source),
            current_wait: None,
            wait_fn,
        }
    }
}

impl<Src: Source<Time = Instant>, Fut: Future<Output = ()>> Stream for InterruptStream<Src, Fut> {
    type Item = (Instant, Interrupt<Src::Event>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let poll_time = Instant::now();
        let poll = this.source.poll_events(poll_time, cx.waker().clone());

        let poll = match poll {
            Ok(poll) => poll,
            Err(_) => panic!(),
        };

        let next_event_at = match poll {
            SourcePoll::Ready {
                state: _,
                next_event_at,
            } => next_event_at,
            SourcePoll::Interrupt {
                interrupt: Interrupt::Complete,
                ..
            } => return Poll::Ready(None),
            SourcePoll::Interrupt { time, interrupt } => {
                return Poll::Ready(Some((time, interrupt)))
            }
            SourcePoll::Pending => return Poll::Pending,
        };

        let next_event_at = match next_event_at {
            Some(next) => next,
            None => {
                this.current_wait = None;
                return Poll::Pending;
            }
        };

        let mut fut = Box::pin((this.wait_fn)(next_event_at));

        match fut.as_mut().poll(cx) {
            Poll::Ready(()) => {
                this.current_wait = None;
                // recurse if the wait immediately returns (shouldn't really happen)
                Pin::new(this).poll_next(cx)
            }
            Poll::Pending => {
                this.current_wait = Some(fut);
                Poll::Pending
            }
        }
    }
}
