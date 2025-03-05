/// A modified version of [`futures::task::Poll`], which has two new variants:
/// 
#[derive(Debug)]
pub enum SourcePoll<T, E, S> {
    /// Indicates the poll is complete
    Ready {
        /// The requested state
        state: S,
        /// The time of the next known event, if known.
        next_event_at: Option<T>,
    },

    /// Indicates information must be handled before state is emitted
    Interrupt {
        /// The time the information pertains to
        time: T,

        /// The type of interrupt
        interrupt: Interrupt<E>,
    },

    /// pending operation. caller will be woken up when progress can be made
    /// the channel this poll used must be retained.
    Pending,
}

impl<T, E, S> SourcePoll<T, E, S> {
    pub fn map_state<F, U>(self, f: F) -> SourcePoll<T, E, U>
    where
        F: FnOnce(S) -> U,
    {
        match self {
            SourcePoll::Ready { state, next_event_at } => SourcePoll::Ready {
                state: f(state),
                next_event_at,
            },
            SourcePoll::Interrupt { time, interrupt } => SourcePoll::Interrupt {
                time,
                interrupt,
            },
            SourcePoll::Pending => SourcePoll::Pending,
        }
    }
}


#[derive(Debug)]
/// The type of interrupt emitted from the source
pub enum Interrupt<E> {
    /// A new event is available.
    Event(E),

    /// An event followed by a finalize, for convenience.
    /// This should be identical to returning an event then a finalize for the same time.
    /// Useful for sources which never emit Rollbacks, so they can simply emit this interrupt
    /// for every event and nothing else.
    FinalizedEvent(E),

    /// All events before at or after time T must be discarded.
    Rollback,
    /// No event will ever be emitted before time T again.
    Finalize,

    /// No interrupt will ever be emitted ever again.
    /// The associated time is the time of the last emitted event.
    Complete,
}

impl<E> Interrupt<E> {
    pub fn map_event<F, U>(self, f: F) -> Interrupt<U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            Interrupt::Event(e) => Interrupt::Event(f(e)),
            Interrupt::FinalizedEvent(e) => Interrupt::FinalizedEvent(f(e)),
            Interrupt::Rollback => Interrupt::Rollback,
            Interrupt::Finalize => Interrupt::Finalize,
            Interrupt::Complete => Interrupt::Complete,
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum SourcePollErr {
    OutOfBoundsChannel,
    PollAfterAdvance,
    PollBeforeDefault,
    SpecificError(anyhow::Error),
}

pub type TrySourcePoll<T, E, S> = Result<SourcePoll<T, E, S>, SourcePollErr>;
