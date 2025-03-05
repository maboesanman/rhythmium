use std::{collections::BTreeSet, task::Waker};

use archery::ArcTK;

use crate::{
    source::{
        source_poll::{Interrupt, TrySourcePoll},
        traits::SourceContext,
        Source, SourcePoll,
    },
    transposer::{input_erasure::ErasedInputState, step::BoxedInput, Transposer},
};

use super::erased_input_source_collection::{ErasedInputSourceCollection, ErasedInputSourceGuard};

#[derive(Debug, Clone)]
pub struct InputSourceMetaData<T: Transposer + 'static> {
    next_scheduled_time: Option<T::Time>,
    finalized_time: Option<T::Time>,
    complete: bool,
    observed_times: BTreeSet<T::Time>,
}

impl<T: Transposer + 'static> Default for InputSourceMetaData<T> {
    fn default() -> Self {
        Self {
            next_scheduled_time: None,
            finalized_time: None,
            complete: false,
            observed_times: BTreeSet::new(),
        }
    }
}

impl<T: Transposer + 'static> InputSourceMetaData<T> {
    pub fn next_scheduled_time(&self) -> Option<T::Time> {
        self.next_scheduled_time
    }

    // pub fn might_interrupt(&self, time: T::Time) -> bool {
    //     match self.next_scheduled_time {
    //         Some(next) => next <= time,
    //         None => false,
    //     }
    // }

    pub fn finalized_time(&self) -> Option<T::Time> {
        self.finalized_time
    }

    pub fn complete(&self) -> bool {
        self.complete
    }
}

impl<T: Transposer + 'static> ErasedInputSourceGuard<'_, T, InputSourceMetaData<T>> {
    // only returns none when an unobserved interrupt occurs.
    fn poll_inner<'b, S>(
        &mut self,
        poll: SourcePoll<T::Time, BoxedInput<'b, T>, S>,
        poll_time: T::Time,
        forget: bool,
    ) -> Option<SourcePoll<T::Time, BoxedInput<'b, T>, S>> {
        let metadata = self.get_metadata_mut();

        match &poll {
            SourcePoll::Ready { next_event_at, .. } => {
                if !forget {
                    metadata.observed_times.insert(poll_time);
                }
                metadata.next_scheduled_time = *next_event_at;
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Event(_),
            } => {
                metadata.observed_times.insert(*time);
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Finalize,
            } => {
                // finalize should remove all observed times before the finalized time.
                metadata.finalized_time = Some(*time);
                metadata.observed_times = metadata.observed_times.split_off(time);
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::FinalizedEvent(_),
            } => {
                metadata.observed_times.insert(*time);
                // finalize should remove all observed times before the finalized time.

                metadata.finalized_time = Some(*time);
                metadata.observed_times = metadata.observed_times.split_off(time);
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Rollback,
            } => {
                // throw out all observed times at or after the rollback time, and return a rollback
                // with the first observed time after the rollback time.
                return metadata.observed_times.split_off(time).first().map(|t| {
                    SourcePoll::Interrupt {
                        time: *t,
                        interrupt: Interrupt::Rollback,
                    }
                });
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Complete,
            } => {
                metadata.complete = true;
                metadata.finalized_time = Some(*time);
                metadata.observed_times.clear();
            }
            SourcePoll::Pending => {}
        }

        Some(poll)
    }
}

impl<T: Transposer + 'static> Source for ErasedInputSourceGuard<'_, T, InputSourceMetaData<T>> {
    type Time = T::Time;

    type Event = BoxedInput<'static, T, ArcTK>;

    type State = Box<ErasedInputState<T>>;

    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        loop {
            let poll = self.get_source_mut().poll(time, cx.clone())?;
            if let Some(p) = self.poll_inner(poll, time, false) {
                break Ok(p);
            };
        }
    }

    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        loop {
            let poll = self.get_source_mut().poll_forget(time, cx.clone())?;
            if let Some(p) = self.poll_inner(poll, time, true) {
                break Ok(p);
            };
        }
    }

    fn poll_events(
        &mut self,
        time: Self::Time,
        waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        loop {
            let poll = self.get_source_mut().poll_events(time, waker.clone())?;
            if let Some(p) = self.poll_inner(poll, time, true) {
                break Ok(p);
            };
        }
    }

    fn release_channel(&mut self, channel: usize) {
        self.get_source_mut().release_channel(channel)
    }

    fn advance(&mut self, time: Self::Time) {
        self.get_source_mut().advance(time)
    }

    fn advance_final(&mut self) {
        self.get_source_mut().advance_final();
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        self.get_source().max_channel()
    }
}

impl<T: Transposer + 'static> ErasedInputSourceCollection<T, InputSourceMetaData<T>> {
    /// call advance on the input sources as needed based on the advance time and the finalize time of other steps.
    pub fn handle_advance_and_finalize(&mut self, advance_time: T::Time) {
        if let Some(Some(time_to_advance_to)) = self
            .iter()
            .filter(|(_, m)| !m.complete)
            .map(|(_, m)| m.finalized_time)
            .chain(Some(Some(advance_time)))
            .min()
        {
            for mut item in self.iter_mut() {
                item.advance(time_to_advance_to);
            }
        }
    }
}
