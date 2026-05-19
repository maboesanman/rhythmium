use core::num::NonZeroUsize;
use core::task::Waker;
use std::task::Poll;

use crate::source::source_poll::{LowerBound, TrySourcePoll, UpperBound};

/// An interface for querying partially complete sources of [states](`Source::State`) and [events](`Source::Events`)
///
/// The [`Source`] trait is the core abstraction for the entire cozal library. Everything is designed around the idea of making chains of [`Source`]s
///
/// When a type implements Source, it models two things:
///
/// - A timestamped set of events
///
/// - A function (in the mathematical sense) mapping [`Time`](`Source::Time`) to [`State`](`Source::State`)
pub trait Source {
    /// The type used for timestamping events and states.
    type Time: Ord + Copy + 'static;

    /// The type of events emitted by the source.
    type Event;

    /// The type of states emitted by the source.
    type State;

    /// Poll for interrupts within the range `interrupt_upper_bound..interrupt_upper_bound`.
    ///
    /// The caller only has control over the upper bound, so it may be more useful to think of this as polling for all remaining interrupts in the range `..interrupt_upper_bound`, though any interrupt which is not in the range `interrupt_upper_bound..poll_lower_bound` is considered undefined behavior.
    fn poll(
        &mut self,
        waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State>;

    /// Update the current obligations for poll.
    /// 
    /// poll must be called immediately after calling this.
    /// 
    /// both state_request_lower_bound and interrupt_upper_bound must be monotonic with respect to subsequent calls of this function, and if state_request_lower_bound is not less than interrupt_upper_bound, no state may ever be requested.
    /// 
    /// requested_states is the complete set of times for which an Interrupt::RequestedState must be emitted before poll may be considered to be Ready.
    /// 
    /// the initial query is always (LowerBound::min(), UpperBound::min(), requestedStated: &[])
    fn update_query(
        &mut self,
        state_request_lower_bound: LowerBound<Self::Time>,
        interrupt_upper_bound: UpperBound<Self::Time>,
        requested_states: &[Self::Time], // T must be unique, sorted, and bounded between min(state_request_lower_bound, interrupt_lower_bound) and interrupt_upper_bound
    ) -> Result<(), ()>;
}
