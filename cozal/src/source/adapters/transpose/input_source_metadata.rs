use std::{
    collections::BTreeSet,
    task::{Poll, Waker},
};

use archery::ArcTK;

use crate::{
    source::{
        Source, SourcePoll,
        source_poll::{Interrupt, LowerBound, SourceBound, TrySourcePoll, UpperBound},
        traits::SourceContext,
    },
    transposer::{Transposer, input_erasure::ErasedInputState, step::BoxedInput},
};

use super::erased_input_source_collection::{ErasedInputSourceCollection, ErasedInputSourceGuard};

#[derive(Debug, Clone)]
pub struct InputSourceMetaData<T: Transposer + 'static> {
    next_scheduled_time: UpperBound<T::Time>,
    observed_times: BTreeSet<T::Time>,
    finalized_lower_bound: LowerBound<T::Time>,
    advanced_lower_bound: LowerBound<T::Time>,
    advanced_upper_bound: UpperBound<T::Time>,
}

impl<T: Transposer + 'static> Default for InputSourceMetaData<T> {
    fn default() -> Self {
        Self {
            next_scheduled_time: UpperBound::min(),
            observed_times: BTreeSet::new(),
            finalized_lower_bound: LowerBound::min(),
            advanced_lower_bound: LowerBound::min(),
            advanced_upper_bound: UpperBound::min(),
        }
    }
}

impl<T: Transposer + 'static> InputSourceMetaData<T> {
    pub fn next_scheduled_time(&self) -> UpperBound<T::Time> {
        self.next_scheduled_time
    }

    fn record_interrupt_lower_bound(&mut self, bound: LowerBound<T::Time>) {
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
        F: FnOnce(&mut Self, &mut S),
    {
        // next scheduled time needs to be min unless our interrupts are resolved,
        // so set it here and we'll set it back if we get stateprogress.
        self.next_scheduled_time = UpperBound::min();

        match poll {
            SourcePoll::StateProgress {
                state,
                next_event_at,
                ..
            } => {
                if let Some(t) = next_event_at {
                    debug_assert!(!self.advanced_upper_bound.test(t))
                }

                self.next_scheduled_time = match next_event_at {
                    Some(t) => UpperBound::inclusive(*t),
                    None => UpperBound::max(),
                };

                state_ready_fn(self, state);
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
                interrupt_lower_bound,
            } => {
                debug_assert!(self.finalized_lower_bound.test(time));

                match self.observed_times.split_off(time).first().copied() {
                    Some(new_time) => {
                        *poll = SourcePoll::Interrupt {
                            time: new_time,
                            interrupt: Interrupt::Rollback,
                            interrupt_lower_bound: *interrupt_lower_bound,
                        }
                    }
                    None => return false,
                }
            }
            _ => {}
        }

        if let Some(interrupt_lower_bound) = poll.get_interrupt_lower_bound() {
            self.record_interrupt_lower_bound(interrupt_lower_bound);
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
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>> {
        loop {
            let mut poll = self.get_source_mut().poll(poll_time, cx.clone())?;

            let metadata = self.get_metadata_mut();
            metadata.resolve_new_poll_time(poll_time);
            if metadata.resolve_poll(&mut poll, |m, state| {
                if state.is_ready() {
                    m.observed_times.insert(poll_time);
                }
            }) {
                break Ok(poll);
            }
        }
    }

    fn poll_forget(
        &mut self,
        poll_time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>> {
        loop {
            let mut poll = self.get_source_mut().poll_forget(poll_time, cx.clone())?;

            let metadata = self.get_metadata_mut();
            metadata.resolve_new_poll_time(poll_time);
            if metadata.resolve_poll(&mut poll, |_, _| {}) {
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

            if self.get_metadata_mut().resolve_poll(&mut poll, |_, _| {}) {
                break Ok(poll);
            }
        }
    }
    
    fn advance_poll_lower_bound(
        &mut self,
        poll_lower_bound: LowerBound<Self::Time>,
    ) {
        let metadata = self.get_metadata_mut();

        if metadata.advanced_lower_bound > poll_lower_bound {
            metadata.advanced_lower_bound = poll_lower_bound;
            self.get_source_mut().advance_poll_lower_bound(poll_lower_bound);
        }
    }
    
    fn advance_interrupt_upper_bound(
        &mut self,
        interrupt_upper_bound: UpperBound<Self::Time>,
        interrupt_waker: Waker,
    ) {
        let metadata = self.get_metadata_mut();

        if metadata.advanced_upper_bound > interrupt_upper_bound {
            metadata.advanced_upper_bound = interrupt_upper_bound;
            self.get_source_mut().advance_interrupt_upper_bound(interrupt_upper_bound, interrupt_waker);
        }
    }

    fn release_channel(&mut self, channel: usize) {
        self.get_source_mut().release_channel(channel)
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        self.get_source().max_channel()
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

    pub fn next_event_upper_bound(&self) -> UpperBound<T::Time> {
        self.iter()
            .map(|(_, m)| m.next_scheduled_time)
            .min()
            .unwrap_or(UpperBound::max())
    }

    pub fn advance_poll_lower_bound_all(
        &mut self,
        poll_lower_bound: LowerBound<T::Time>,
    ) {
        self.iter_mut()
            .for_each(|mut x| x.advance_poll_lower_bound(poll_lower_bound));
    }

    pub fn advance_interrupt_upper_bound(
        &mut self,
        interrupt_upper_bound: UpperBound<T::Time>,
        interrupt_waker: Waker,
    ) {
        self.iter_mut()
            .for_each(|mut x| x.advance_interrupt_upper_bound(interrupt_upper_bound, interrupt_waker.clone()));
    }
}
