use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{FutureExt, Stream};
use smallvec::{SmallVec, smallvec};

use crate::source::{
    Source, SourcePoll,
    source_poll::{Interrupt, LowerBound, UpperBound},
};

pub struct EventStream<Src: Source, W, F> {
    source: Src,
    source_finalize_high_point: LowerBound<Src::Time>,
    buffered_events: BTreeMap<Src::Time, SmallVec<[Src::Event; 1]>>,
    start_time: Instant,
    waiting_future: Option<W>,
    wait_fn: F,
}

impl<Src: Source<Time = Duration>, W, F> EventStream<Src, W, F>
where
    W: Future<Output = ()> + Unpin,
    F: FnMut(Instant) -> W,
{
    pub fn new(source: Src, start_time: Instant, wait_fn: F) -> Self {
        Self {
            source,
            source_finalize_high_point: LowerBound::min(),
            buffered_events: BTreeMap::new(),
            start_time,
            waiting_future: None,
            wait_fn,
        }
    }
}

impl<Src: Source<Time = Duration>, W, F> Stream for EventStream<Src, W, F>
where
    W: Future<Output = ()> + Unpin,
    F: FnMut(Instant) -> W,
    Src::Event: Debug,
{
    type Item = (Src::Time, SmallVec<[Src::Event; 1]>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let now = Instant::now();
        let poll_time = now - self.start_time;
        let this = unsafe { self.get_unchecked_mut() };
        this.waiting_future = None;

        loop {
            if let Some(entry) = this.buffered_events.first_entry() {
                if !this.source_finalize_high_point.test(entry.key()) {
                    return Poll::Ready(Some(entry.remove_entry()));
                }
            } else if this.source_finalize_high_point == LowerBound::max() {
                return Poll::Ready(None);
            }

            if let Some(fut) = &mut this.waiting_future {
                let _ = fut.poll_unpin(cx);
            }

            this.source.advance(
                LowerBound::inclusive(poll_time),
                UpperBound::inclusive(poll_time),
                cx.waker().clone(),
            );

            let poll = this.source.poll_interrupts(cx.waker().clone()).unwrap();

            if let Some(x) = poll.get_finalize_bound() {
                this.source_finalize_high_point = x;
            }

            match poll {
                SourcePoll::StateProgress {
                    state: (),
                    next_event_at,
                    ..
                } => {
                    if this.source_finalize_high_point == LowerBound::max()
                        && this.buffered_events.is_empty()
                    {
                        return Poll::Ready(None);
                    }
                    if let Some(t) = next_event_at {
                        this.waiting_future = Some((this.wait_fn)(this.start_time + t));
                    }
                }
                SourcePoll::Interrupt {
                    time,
                    interrupt: Interrupt::Event(event),
                    ..
                } => match this.buffered_events.entry(time) {
                    Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(smallvec![event]);
                    }
                    Entry::Occupied(mut occupied_entry) => {
                        occupied_entry.get_mut().push(event);
                    }
                },
                SourcePoll::Interrupt {
                    time,
                    interrupt: Interrupt::Rollback,
                    ..
                } => {
                    this.buffered_events.split_off(&time);
                }
                SourcePoll::Finalize { .. } => {}
                SourcePoll::InterruptPending => return Poll::Pending,
            }
        }
    }
}
