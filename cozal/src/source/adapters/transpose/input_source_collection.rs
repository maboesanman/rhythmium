use std::task::Waker;

use archery::ArcTK;

use crate::{
    source::{
        source_poll::{Interrupt, LowerBound, UpperBound}, traits::SourceContext, SourcePoll
    },
    transposer::{input_erasure::ErasedInputState, step::BoxedInput, Transposer},
};

use super::erased_input_source_collection::ErasedInputSourceCollection;

pub struct InputSourceCollection<T: Transposer + 'static> {
    pub inputs: ErasedInputSourceCollection<T, ()>,
}

impl<T: Transposer + 'static> InputSourceCollection<T> {
    /// Advance all interrupts to the specified upper bound
    pub fn advance_interrupt_upper_bound<F>(
        &mut self,
        interrupt_upper_bound: UpperBound<T::Time>,
        mut interrupt_waker_fn: F,
    ) where F: FnMut(u64) -> Waker
    {
        self.inputs.iter_mut_with_hashes().for_each(move |(hash, mut source)| {
            let interrupt_waker = interrupt_waker_fn(hash);
            source.get_source_mut().advance_interrupt_upper_bound(interrupt_upper_bound, interrupt_waker);
        });
    }

    /// register the poll lower bound from the caller.
    pub fn advance_poll_lower_bound(&mut self, lower_bound: LowerBound<T::Time>) {
        // todo!()
    }

    /// return the min of the returned lower bounds of all the inputs.
    pub fn get_input_interrupt_lower_bound(&self) -> LowerBound<T::Time> {
        todo!()
    }

    /// inform the input collection of the lower bound for when steps might request state
    pub fn set_tentative_request_state_lower_bound(&mut self, lower_bound: LowerBound<T::Time>) {
        todo!()
    }

    /// poll until all interrupts are returning StateProgress
    pub fn poll_aggregate_interrupts<F>(
        &mut self,
        mut interrupt_waker_fn: F,
    ) -> AggregateSourcePoll<T>  where F: FnMut(u64) -> Waker
    {
        // todo!()
        AggregateSourcePoll::StateProgress { next_event_at: None, interrupt_lower_bound: LowerBound::max() }
    }

    /// poll one specific input for state
    pub fn poll_single(
        &mut self,
        input_hash: u64,
        time: T::Time,
        cx: SourceContext,
        forget: bool,
    ) -> SourcePoll<T::Time, BoxedInput<'static, T, ArcTK>, Box<ErasedInputState<T>>> {
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