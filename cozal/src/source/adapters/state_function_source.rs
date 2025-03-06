use std::{marker::PhantomData, num::NonZeroUsize};

use crate::source::{
    source_poll::{Interrupt, TrySourcePoll},
    traits::SourceContext,
    Source, SourcePoll,
};

pub struct StateFunctionSource<T, S, F> {
    first_call: bool,
    function: F,

    phantom: PhantomData<fn(T) -> S>,
}

impl<T, S, F> StateFunctionSource<T, S, F>
where
    F: FnMut(T) -> S,
{
    pub fn new(function: F) -> Self {
        Self {
            first_call: false,
            function,
            phantom: PhantomData,
        }
    }
}

impl<T: Ord + Copy, S, F> Source for StateFunctionSource<T, S, F>
where
    F: FnMut(T) -> S,
{
    type Time = T;

    type Event = ();

    type State = S;

    fn poll(
        &mut self,
        time: Self::Time,
        _cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        if self.first_call {
            self.first_call = false;
            Ok(SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Complete,
            })
        } else {
            Ok(SourcePoll::Ready {
                state: (self.function)(time),
                next_event_at: None,
            })
        }
    }

    fn poll_events(
        &mut self,
        time: Self::Time,
        _interrupt_waker: std::task::Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        if self.first_call {
            self.first_call = false;
            Ok(SourcePoll::Interrupt {
                time,
                interrupt: Interrupt::Complete,
            })
        } else {
            Ok(SourcePoll::Ready {
                state: (),
                next_event_at: None,
            })
        }
    }

    fn release_channel(&mut self, _channel: usize) {
        // noop
    }

    fn advance(&mut self, _time: Self::Time) {
        // noop
    }

    fn advance_final(&mut self) {
        // noop
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        NonZeroUsize::MAX
    }
}
