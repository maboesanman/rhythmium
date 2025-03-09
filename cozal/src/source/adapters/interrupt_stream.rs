use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use futures::Stream;

use crate::source::source_poll::{LowerBound, UpperBound};

use super::super::source_poll::Interrupt;
use super::super::Source;

pub struct RealtimeInterruptStream<Src: Source<Time = Instant>, Fut: Future<Output = ()>> {
    source: Box<Src>,
    current_wait: Option<Pin<Box<Fut>>>,
    wait_fn: fn(Instant) -> Fut,
}

impl<Src: Source<Time = Instant>, Fut: Future<Output = ()>> RealtimeInterruptStream<Src, Fut> {
    pub fn new(source: Src, wait_fn: fn(Instant) -> Fut) -> Self {
        Self {
            source: Box::new(source),
            current_wait: None,
            wait_fn,
        }
    }
}

impl<Src: Source<Time = Instant>, Fut: Future<Output = ()>> Stream
    for RealtimeInterruptStream<Src, Fut>
{
    type Item = (Instant, Interrupt<Src::Event>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let now = Instant::now();
        self.source.advance(
            LowerBound::exclusive(now),
            UpperBound::inclusive(now),
            cx.waker().clone(),
        );
        todo!()
        // let this = self.get_mut();
        // let now = Instant::now();
        // this.source.advance(Bound::Excluded(now), Bound::Included(now));
        // let poll = this.source.poll_interrupts(cx.waker().clone());

        // let poll = match poll {
        //     Ok(poll) => poll,
        //     Err(_) => panic!(),
        // };

        // let next_event_at = match poll {
        //     SourcePoll::Ready {
        //         state: _,
        //         next_event_at,
        //     } => next_event_at,
        //     SourcePoll::Interrupt {
        //         interrupt: Interrupt::Finalize(Bound::Unbounded),
        //         ..
        //     } => return Poll::Ready(None),
        //     SourcePoll::Interrupt { time, interrupt } => {
        //         return Poll::Ready(Some((time, interrupt)))
        //     }
        //     SourcePoll::Pending => return Poll::Pending,
        // };

        // let next_event_at = match next_event_at {
        //     Some(next) => next,
        //     None => {
        //         this.current_wait = None;
        //         return Poll::Pending;
        //     }
        // };

        // let mut fut = Box::pin((this.wait_fn)(next_event_at));

        // match fut.as_mut().poll(cx) {
        //     Poll::Ready(()) => {
        //         this.current_wait = None;
        //         // recurse if the wait immediately returns (shouldn't really happen)
        //         Pin::new(this).poll_next(cx)
        //     }
        //     Poll::Pending => {
        //         this.current_wait = Some(fut);
        //         Poll::Pending
        //     }
        // }
    }
}
