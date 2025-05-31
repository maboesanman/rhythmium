use std::{
    collections::BTreeMap,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{FutureExt, Stream, StreamExt};

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

            core::mem::replace(buffer, at_or_after)
        }
        SourceBound::Exclusive(t) => {
            let mut after = buffer.split_off(&t);
            if let Some(x) = after.remove(&t) {
                buffer.insert(t, x);
            }

            core::mem::replace(buffer, after)
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

                    if next_event_at.is_some() && interrupt_lower_bound == LowerBound::max() {
                        panic!()
                    }

                    if this.buffered_events.is_empty() {
                        if interrupt_lower_bound == LowerBound::max() {
                            return Poll::Ready(None);
                        }

                        if let Some(t) = next_event_at {
                            this.source.advance_interrupt_upper_bound(
                                UpperBound::inclusive(t),
                                cx.waker().clone(),
                            );
                            // we loop to the beginning and try repolling.
                        } else {
                            // just gotta wait for more events.
                            return Poll::Pending;
                        }
                    }
                }
                SourcePoll::Interrupt {
                    time,
                    interrupt,
                    interrupt_lower_bound,
                } => {
                    match interrupt {
                        Interrupt::Event(event) => {
                            this.buffered_events.entry(time).or_default().push(event)
                        }
                        Interrupt::Rollback => drop(this.buffered_events.split_off(&time)),
                    }

                    let to_emit =
                        prune_by_lower_bound(&mut this.buffered_events, interrupt_lower_bound);
                    if !to_emit.is_empty() {
                        return Poll::Ready(Some(to_emit));
                    }
                    // we loop to the beginning and try repolling, since we need to poll after interrupts.
                }
                SourcePoll::InterruptPending => return Poll::Pending,
            }
        }
    }
}
