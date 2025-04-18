use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt::Debug,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures::{FutureExt, Stream, StreamExt};
use smallvec::{SmallVec, smallvec};

use crate::source::{
    Source, SourcePoll,
    source_poll::{Interrupt, LowerBound, SourceBound, UpperBound},
};

pub fn into_event_stream<Src: Source>(source: Src) -> impl Stream<Item = (Src::Time, Src::Event)> {
    EventStreamRaw::new(source).flat_map(|x| {
        futures::stream::iter(
            x.into_iter()
                .flat_map(|(t, events)| events.into_iter().map(move |e| (t, e))),
        )
    })
}


struct EventStreamRaw<Src: Source> {
    source: Src,
    buffered_events: BTreeMap<Src::Time, Vec<Src::Event>>,
}


impl<Src: Source> EventStreamRaw<Src> {
    pub fn new(mut source: Src) -> Self {
        source.advance_poll_lower_bound(LowerBound::max());
        Self {
            source,
            buffered_events: BTreeMap::new(),
        }
    }
}

fn prune_by_lower_bound<T: Ord, E>(
    buffer: &mut BTreeMap<T, Vec<E>>,
    lower_bound: LowerBound<T>,
) -> BTreeMap<T, Vec<E>> {
    match lower_bound.0 {
        SourceBound::Min => BTreeMap::new(),
        SourceBound::Inclusive(t) => {
            let at_or_after = buffer.split_off(&t);
            let before = core::mem::replace(buffer, at_or_after);
            before
        }
        SourceBound::Exclusive(t) => {
            let mut after = buffer.split_off(&t);
            if let Some(x) = after.remove(&t) {
                buffer.insert(t, x);
            }
            let before = core::mem::replace(buffer, after);
            before
        }
        SourceBound::Max => core::mem::take(buffer),
    }
}

impl<Src: Source> Stream for EventStreamRaw<Src> {
    type Item = BTreeMap<Src::Time, Vec<Src::Event>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            match this.source.poll_interrupts(cx.waker().clone()).unwrap() {
                SourcePoll::StateProgress {
                    state: (),
                    next_event_at,
                    interrupt_lower_bound,
                } => {
                    let to_emit =
                        prune_by_lower_bound(&mut this.buffered_events, interrupt_lower_bound);
                    if !to_emit.is_empty() {
                        return Poll::Ready(Some(to_emit));
                    }

                    if interrupt_lower_bound == LowerBound::max() && this.buffered_events.is_empty() {
                        return Poll::Ready(None);
                    }

                    if this.buffered_events.is_empty() {
                        if let Some(t) = next_event_at {
                            this.source.advance_interrupt_upper_bound(
                                UpperBound::inclusive(t),
                                cx.waker().clone(),
                            );
                        } else {
                            println!("empty")
                        }
                    }
                }
                SourcePoll::Interrupt {
                    time,
                    interrupt,
                    interrupt_lower_bound,
                } => {
                    match interrupt {
                        Interrupt::Event(event) => match this.buffered_events.entry(time) {
                            Entry::Vacant(vacant_entry) => {
                                vacant_entry.insert(vec![event]);
                            }
                            Entry::Occupied(occupied_entry) => {
                                occupied_entry.into_mut().push(event);
                            }
                        },
                        Interrupt::Rollback => {
                            this.buffered_events.split_off(&time);
                        }
                    }

                    let to_emit =
                        prune_by_lower_bound(&mut this.buffered_events, interrupt_lower_bound);
                    if !to_emit.is_empty() {
                        return Poll::Ready(Some(to_emit));
                    }
                }
                SourcePoll::InterruptPending => return Poll::Pending,
            }
        }
    }
}
