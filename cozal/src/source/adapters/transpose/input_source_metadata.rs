use std::{collections::BTreeSet, task::Waker};

use archery::ArcTK;

use crate::{
    source::{
        source_poll::{Interrupt, LowerBound, SourceBound, TrySourcePoll, UpperBound},
        traits::SourceContext,
        Source, SourcePoll,
    },
    transposer::{input_erasure::ErasedInputState, step::BoxedInput, Transposer},
};

use super::erased_input_source_collection::{ErasedInputSourceCollection, ErasedInputSourceGuard};

#[derive(Debug, Clone)]
pub struct InputSourceMetaData<T: Transposer + 'static> {
    next_scheduled_time: Option<T::Time>,
    observed_times: BTreeSet<T::Time>,
    finalized_lower_bound: LowerBound<T::Time>,
    advanced_lower_bound: LowerBound<T::Time>,
    advanced_upper_bound: UpperBound<T::Time>,
}

impl<T: Transposer + 'static> Default for InputSourceMetaData<T> {
    fn default() -> Self {
        Self {
            next_scheduled_time: None,
            observed_times: BTreeSet::new(),
            finalized_lower_bound: LowerBound::min(),
            advanced_lower_bound: LowerBound::min(),
            advanced_upper_bound: UpperBound::min(),
        }
    }
}

impl<T: Transposer + 'static> InputSourceMetaData<T> {
    pub fn next_scheduled_time(&self) -> Option<T::Time> {
        self.next_scheduled_time
    }

    fn record_finalize_bound(&mut self, bound: LowerBound<T::Time>) {
        debug_assert!(self.finalized_lower_bound < bound);

        self.finalized_lower_bound = bound;
        match self.finalized_lower_bound.0 {
            SourceBound::Min => {}
            SourceBound::Inclusive(t) => {
                self.observed_times = self.observed_times.split_off(&t);
            }
            SourceBound::Exclusive(t) => {
                self.observed_times = self.observed_times.split_off(&t);
                self.observed_times.remove(&t);
            }
            SourceBound::Max => {
                self.observed_times.clear();
            }
        }
    }

    fn resolve_new_poll_time(&mut self, poll_time: T::Time) {
        debug_assert!(self.advanced_lower_bound.test(&poll_time));
        self.advanced_upper_bound = self
            .advanced_upper_bound
            .min(UpperBound::inclusive(poll_time));
    }

    // returns true if poll should be returned, or false if it should be skipped (only for rollbacks that were never observed)
    fn resolve_poll<E, S, F>(
        &mut self,
        poll: &mut SourcePoll<T::Time, E, S>,
        state_ready_fn: F,
    ) -> bool
    where
        F: FnOnce(&mut Self),
    {
        match poll {
            SourcePoll::StateProgress {
                state,
                next_event_at,
                ..
            } => {
                if let Some(t) = next_event_at {
                    debug_assert!(!self.advanced_upper_bound.test(t))
                }

                self.next_scheduled_time = *next_event_at;

                if state.is_ready() {
                    state_ready_fn(self);
                }
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Event(_),
                ..
            } => {
                debug_assert!(self.finalized_lower_bound.test(time));
                self.observed_times.insert(*time);
            }
            SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Rollback,
                finalize_bound,
            } => {
                debug_assert!(self.finalized_lower_bound.test(time));
                self.next_scheduled_time = None;

                match self.observed_times.split_off(time).first().copied() {
                    Some(new_time) => {
                        *poll = SourcePoll::Interrupt {
                            time: new_time,
                            interrupt: Interrupt::Rollback,
                            finalize_bound: *finalize_bound,
                        }
                    }
                    None => return false,
                }
            }
            SourcePoll::Finalize { .. } => {}
            SourcePoll::InterruptPending => {}
        }

        if let Some(finalize_bound) = poll.get_finalize_bound() {
            self.record_finalize_bound(finalize_bound);
        }

        true
    }
}

impl<T: Transposer + 'static> Source for ErasedInputSourceGuard<'_, T, InputSourceMetaData<T>> {
    type Time = T::Time;

    type Event = BoxedInput<'static, T, ArcTK>;

    type State = Box<ErasedInputState<T>>;

    fn poll(
        &mut self,
        poll_time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        loop {
            let mut poll = self.get_source_mut().poll(poll_time, cx.clone())?;

            let metadata = self.get_metadata_mut();
            metadata.resolve_new_poll_time(poll_time);
            if metadata.resolve_poll(&mut poll, |m| {
                m.observed_times.insert(poll_time);
            }) {
                break Ok(poll);
            }
        }
    }

    fn poll_forget(
        &mut self,
        poll_time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        loop {
            let mut poll = self.get_source_mut().poll_forget(poll_time, cx.clone())?;

            let metadata = self.get_metadata_mut();
            metadata.resolve_new_poll_time(poll_time);
            if metadata.resolve_poll(&mut poll, |_| {}) {
                break Ok(poll);
            }
        }
    }

    fn poll_interrupts(
        &mut self,
        interrupt_waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        loop {
            let mut poll = self
                .get_source_mut()
                .poll_interrupts(interrupt_waker.clone())?;

            if self.get_metadata_mut().resolve_poll(&mut poll, |_| {}) {
                break Ok(poll);
            }
        }
    }

    fn release_channel(&mut self, channel: usize) {
        self.get_source_mut().release_channel(channel)
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        self.get_source().max_channel()
    }

    fn advance(
        &mut self,
        lower_bound: LowerBound<Self::Time>,
        upper_bound: UpperBound<Self::Time>,
    ) {
        let metadata = self.get_metadata_mut();

        debug_assert!(metadata.advanced_lower_bound < lower_bound);
        metadata.advanced_lower_bound = lower_bound;

        debug_assert!(metadata.advanced_upper_bound < upper_bound);
        metadata.advanced_upper_bound = upper_bound;

        self.get_source_mut().advance(lower_bound, upper_bound);
    }
}

impl<T: Transposer + 'static> ErasedInputSourceCollection<T, InputSourceMetaData<T>> {
    /// The earliest possible time, if it exists.
    ///
    /// None means there cannot ever be another incoming interrupt.
    /// Some(None) means an interrupt may come from any value of T::Time.
    /// Some(Some(t)) means there will never be an incoming interrupt before t.
    pub fn interupt_lower_bound(&self) -> LowerBound<T::Time> {
        self.iter()
            .map(|x| x.1.finalized_lower_bound)
            .min()
            .unwrap_or(LowerBound::max())
    }

    pub fn first_next_event(&self) -> Option<T::Time> {
        self.iter().filter_map(|x| x.1.next_scheduled_time).min()
    }

    pub fn advance_all(
        &mut self,
        lower_bound: LowerBound<T::Time>,
        upper_bound: UpperBound<T::Time>,
    ) {
        self.iter_mut()
            .for_each(|mut x| x.advance(lower_bound, upper_bound));
    }
}
