use core::num::NonZeroUsize;
use core::task::Waker;
use std::task::Poll;

use crate::source::source_poll::{LowerBound, TrySourcePoll, UpperBound};

#[derive(Clone)]
pub struct SourceContext {
    pub channel: usize,
    pub channel_waker: Waker,
    pub interrupt_waker: Waker,
}

impl SourceContext {
    pub fn change_channel(&mut self, new_channel: usize) {
        self.channel = new_channel;
    }

    pub fn with_interrupt_only(&self) -> Self {
        Self {
            channel: 0,
            channel_waker: self.interrupt_waker.clone(),
            interrupt_waker: self.interrupt_waker.clone(),
        }
    }
}

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

    /// Attempt to retrieve the state of the source at `time`, registering the current task for wakeup in certain situations.
    ///
    /// # Return value
    ///
    /// There are four variants to the `SourcePoll` enum:
    /// 
    /// - `StateProgress{ state, next_event_at, interrupt_lower_bound }`: There are no known interrupts pending before the current `interrupt_upper_bound`. If next_event_at is `Some`, then it must be greater than `interrupt_upper_bound`. 
    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>>;

    /// Attempt to retrieve the state of the source at `time`, registering the current task for wakeup in certain situations. Also inform the source that the state emitted from this call is exempt from the requirement to be informed of future invalidations (that the source can "forget" about this call to poll when determining how far to roll back).
    ///
    /// If you do not need to be notified that this state has been invalidated (if for example you polled in order to render to the screen, so finding out your previous frame was wrong means nothing because you can't take back the photons from the user's eyeballs) then this function should be preferred.
    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>> {
        self.poll(time, cx)
    }

    /// Poll for interrupts within the range `interrupt_upper_bound..interrupt_upper_bound`.
    /// 
    /// The caller only has control over the upper bound, so it may be more useful to think of this as polling for all remaining interrupts in the range `..interrupt_upper_bound`, though any interrupt which is not in the range `interrupt_upper_bound..poll_lower_bound` is considered undefined behavior.
    fn poll_interrupts(
        &mut self,
        interrupt_waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()>;

    /// Inform the source that niether `poll` nor `poll_forget` will be called with a time below`poll_lower_bound`.
    /// 
    /// The caller may use this to inform the source that it is no longer interested in states before `poll_lower_bound`, and that the source may discard any extra data it had been retaining to handle such polls.
    fn advance_poll_lower_bound(
        &mut self,
        poll_lower_bound: LowerBound<Self::Time>,
    );

    /// Inform the source that all known interrupts below `interrupt_upper_bound` must be emitted before any `StateProgress` may be returned.
    /// 
    /// Proper usage of this is critical, because there is no gurantee of the order the interrupts will be returned in. If you do not interleave polling and advancing the interrupt upper bound, you will not have any control over the order of the events returned.
    /// 
    /// There is no lower bound here, because the source is the authority on the finality of events it has emitted, and therefore communicates the lower bound via fields in the poll result.
    fn advance_interrupt_upper_bound(
        &mut self,
        interrupt_upper_bound: UpperBound<Self::Time>,
        interrupt_waker: Waker,
    );

    /// Inform the source it is no longer obligated to retain progress made on `channel`
    fn release_channel(&mut self, channel: usize);

    /// The maximum value which can be used as the channel for a poll call.
    ///
    /// all channels between 0 and max_channel() inclusive can be used as a channel.
    fn max_channel(&self) -> NonZeroUsize;
}
