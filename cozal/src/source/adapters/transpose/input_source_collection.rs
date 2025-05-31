use std::{
    collections::HashMap,
    task::{Poll, Waker},
};

use archery::ArcTK;

use crate::{
    source::{
        SourcePoll,
        source_poll::{Interrupt, LowerBound, UpperBound},
        traits::SourceContext,
    },
    transposer::{Transposer, input_erasure::ErasedInputState, step::BoxedInput},
};

use super::{
    erased_input_source_collection::ErasedInputSourceCollection,
    transpose_interrupt_waker::TransposeInterruptWakerInner,
};

pub struct InputSourceCollection<T: Transposer + 'static> {
    pub inputs: ErasedInputSourceCollection<T, ()>,
    pub next_events_at: HashMap<u64, T::Time>,
    pub interrupt_lower_bounds: HashMap<u64, LowerBound<T::Time>>,
}

impl<T: Transposer + 'static> InputSourceCollection<T> {
    pub fn new(inputs: ErasedInputSourceCollection<T, ()>) -> Self {
        let interrupt_lower_bounds = inputs
            .iter_with_hashes()
            .map(|(h, _, _)| (h, LowerBound::min()))
            .collect();
        Self {
            inputs,
            next_events_at: HashMap::new(),
            interrupt_lower_bounds,
        }
    }

    /// Advance all interrupts to the specified upper bound
    pub fn advance_interrupt_upper_bound<F>(
        &mut self,
        interrupt_upper_bound: UpperBound<T::Time>,
        mut interrupt_waker_fn: F,
    ) where
        F: FnMut(u64) -> Waker,
    {
        self.inputs
            .iter_mut_with_hashes()
            .for_each(move |(hash, mut source)| {
                let interrupt_waker = interrupt_waker_fn(hash);
                source
                    .get_source_mut()
                    .advance_interrupt_upper_bound(interrupt_upper_bound, interrupt_waker);
            });
    }

    /// register the poll lower bound from the caller.
    pub fn advance_poll_lower_bound(&mut self, lower_bound: LowerBound<T::Time>) {
        // todo!()
    }

    /// return the min of the returned lower bounds of all the inputs.
    pub fn get_input_interrupt_lower_bound(&self) -> LowerBound<T::Time> {
        self.interrupt_lower_bounds
            .values()
            .min()
            .copied()
            .unwrap_or(LowerBound::max())
    }

    pub fn get_input_next_event_at(&self) -> Option<T::Time> {
        self.next_events_at.values().min().copied()
    }

    pub fn register_next_event_at(&mut self, input_hash: u64, next_event_at: Option<T::Time>) {
        match next_event_at {
            Some(t) => self.next_events_at.insert(input_hash, t),
            None => self.next_events_at.remove(&input_hash),
        };
    }

    pub fn register_interrupt_lower_bound(
        &mut self,
        input_hash: u64,
        interrupt_lower_bound: LowerBound<T::Time>,
    ) {
        if interrupt_lower_bound == LowerBound::max() {
            self.interrupt_lower_bounds.remove(&input_hash);
        } else {
            self.interrupt_lower_bounds
                .insert(input_hash, interrupt_lower_bound);
        }
    }

    /// inform the input collection of the lower bound for when steps might request state
    pub fn set_tentative_request_state_lower_bound(&mut self, lower_bound: LowerBound<T::Time>) {
        todo!()
    }

    /// poll all woken inputs until they are all ready or pending (no woken).
    pub fn poll_aggregate_interrupts<F>(
        &mut self,
        wakers_inner: &mut TransposeInterruptWakerInner,
        mut interrupt_waker_fn: F,
    ) -> AggregateSourcePoll<T>
    where
        F: FnMut(u64) -> Waker,
    {
        while let Some(input_hash) = wakers_inner.input_interrupt_woken.pop_front() {
            let interrupt_waker = interrupt_waker_fn(input_hash);
            let mut source = self.inputs.get_input_by_hash(input_hash).unwrap();

            let poll = source
                .get_source_mut()
                .poll_interrupts(interrupt_waker)
                .unwrap();

            match poll {
                SourcePoll::StateProgress {
                    state: (),
                    next_event_at,
                    interrupt_lower_bound,
                } => {
                    self.register_interrupt_lower_bound(input_hash, interrupt_lower_bound);
                    self.register_next_event_at(input_hash, next_event_at);
                }
                SourcePoll::Interrupt {
                    time,
                    interrupt,
                    interrupt_lower_bound,
                } => {
                    self.register_interrupt_lower_bound(input_hash, interrupt_lower_bound);
                    wakers_inner.input_interrupt_woken.push_back(input_hash);
                    return AggregateSourcePoll::Interrupt {
                        input_hash,
                        time,
                        interrupt,
                        interrupt_lower_bound: self.get_input_interrupt_lower_bound(),
                    };
                }
                SourcePoll::InterruptPending => {
                    wakers_inner.input_interrupt_pending.push_back(input_hash);
                }
            }
        }

        if wakers_inner.input_interrupt_pending.is_empty() {
            AggregateSourcePoll::StateProgress {
                next_event_at: self.get_input_next_event_at(),
                interrupt_lower_bound: self.get_input_interrupt_lower_bound(),
            }
        } else {
            AggregateSourcePoll::InterruptPending
        }
    }

    /// poll one specific input for state
    pub fn poll_single(
        &mut self,
        input_hash: u64,
        time: T::Time,
        cx: SourceContext,
        forget: bool,
    ) -> SourcePoll<T::Time, BoxedInput<'static, T, ArcTK>, Poll<Box<ErasedInputState<T>>>> {
        todo!()
    }

    /// release one specific input's input channel.
    pub fn release_single_channel(&mut self, input_hash: u64, channel: usize) {
        todo!()
    }
}

/// The return type used by [`Source::poll`], [`Source::poll_forget`] and [`Source::poll_interrupts`] to communicate the current state of the source.
#[derive(Debug, PartialEq, Eq)]
pub enum AggregateSourcePoll<T: Transposer + 'static> {
    StateProgress {
        next_event_at: Option<T::Time>,
        interrupt_lower_bound: LowerBound<T::Time>,
    },
    Interrupt {
        input_hash: u64,
        time: T::Time,
        interrupt: Interrupt<BoxedInput<'static, T, ArcTK>>,
        interrupt_lower_bound: LowerBound<T::Time>,
    },
    InterruptPending,
}
