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
    type Time: Ord + Copy;

    /// The type of events emitted by the source.
    type Event;

    /// The type of states emitted by the source.
    type State;

    /// Attempt to retrieve the state of the source at `time`, registering the current task for wakeup in certain situations.
    ///
    /// # Return value
    ///
    /// There are several possible return values, each indicating a distinct source state for a time `t`:
    ///
    /// - [`Ready(state)`](EventStatePoll::Ready) indicates that all known events have been emitted, and that the state is ready for the requested time. New events must wake the current task in order to be retrieved, though poll may be called at any time.
    ///
    /// - [`Scheduled(state, t_s)`](EventStatePoll::Scheduled) indicates that All events at or before time `t` have been emitted, that the next event ready to be emitted is at time `t_s`, and that the state is ready for the requested time. Emitting `Scheduled` releases the source of the responsibility of waking the current task for any new information which does not change any emitted states or create any events before time `t_s`. It is still responsible for waking the task on changes that affect events or states preceding `t_s`. The source should be polled again at or after `t_s`, though what it means to be "at time t_s" is left to the consumer of the trait.
    ///
    /// - [`Event(payload, t_e)`](EventStatePoll::Event) indicates that the requested state could not be computed because the returned event must be handled before the state can be made available. The source should be immediately polled again, as it may never wake the task.
    ///
    /// - [`Rollback(t_r)`](EventStatePoll::Rollback) indicates that previously emitted information has been discovered to be incorrect, and that the caller should re-poll information it believes it needs. Specifically, all emitted events at or after time `t_r` should be discarded, as well as all states returned from [`poll`](Source::poll). Emitting rollback makes no claims about states returned from [`poll_forget`](Source::poll_forget), which should be preferred when the caller doesn't need to be informed of states being invalidated.
    ///
    /// - [`Pending`](EventStatePoll::Pending) indicates one of the other responses is not available at this time. the current thread will be woken up when progress can be made by calling poll again with the same time. You should be polling at the same time as the call which returned pending if you are responding to a task wake, or else you might not actually be able to make progress.
    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>>;

    /// Attempt to retrieve the state of the source at `time`, registering the current task for wakeup in certain situations. Also inform the source that the state emitted from this call is exempt from the requirement to be informed of future invalidations (that the source can "forget" about this call to poll when determining how far to roll back).
    ///
    /// If you do not need to be notified that this state has been invalidated (if for example you polled in order to render to the screen, so finding out your previous frame was wrong means nothing because you can't go back and change it) then this function should be preferred.
    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>> {
        self.poll(time, cx)
    }

    /// Attempt to determine information about the set of events in the range (..upper_bound) - (..last_finalize_upper_bound).
    fn poll_interrupts(
        &mut self,
        interrupt_waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()>;

    /// Inform the source of the specific range in which poll and poll_forget may be called, and additionally
    /// what time we need to be notified of all events before.
    ///
    /// the range is (..upper_bound) - (..lower_bound) where the subtraction is set subtraction.
    /// note that this is NOT THE SAME as lower_bound..upper_bound. If we had represented it this way, we would not be able to express the completed state properly.
    /// lower bound must be less than or equal to upper bound, and if both are Unbounded, then no event will ever be emitted
    /// ever again.
    ///
    /// subsequent calls must not call lower or upper with values less than the previous call (including implied upper_bounds from poll/poll_forget calls)
    ///
    /// the implication here is after calling this all interrupts will be at times in the range (..upper_bound) - (..last_finalize_upper_bound)
    /// and all state polls will be at times in the range (..upper_bound) - (..lower_bound)
    fn advance(
        &mut self,
        lower_bound: LowerBound<Self::Time>,
        upper_bound: UpperBound<Self::Time>,
        interrupt_waker: Waker,
    );

    /// Inform the source it is no longer obligated to retain progress made on `channel`
    fn release_channel(&mut self, channel: usize);

    /// The maximum value which can be used as the channel for a poll call.
    ///
    /// all channels between 0 and max_channel() inclusive can be used as a channel.
    fn max_channel(&self) -> NonZeroUsize;
}
