use std::task::Poll;

/// The return type used by [`Source::poll`], [`Source::poll_forget`] and [`Source::poll_interrupts`] to communicate the current state of the source.
#[derive(Debug, PartialEq, Eq)]
pub enum SourcePoll<T, E, S> {
    /// Indicates the set of known events in the range `interrupt_lower_bound..interrupt_upper_bound` have all been emitted. Once this has happened the source may begin making progress on events (in the case of `poll` or `poll_forget`) or simply use this variant to communicate that the source has finished emitting events it knows about in the range.
    ///
    /// The source must invoke the interrupt waker if the next_event_at value might change, or if there might be new interrupts.
    ///
    /// The source must invoke the state waker if the state was a Poll::Pending value and might make progress.
    ///
    /// For `poll` and `poll_forget`, S is Poll<Src::OutputState>.
    ///
    /// For `poll_interrupts`, S is ().
    StateProgress {
        /// The requested state, if ready
        /// the channel waker must be called to wake if pending.
        state: S,

        /// The time of the next event after the current advance upper bound.
        next_event_at: Option<T>,

        /// The current finalize bound (after this state is processed)
        interrupt_lower_bound: LowerBound<T>,
    },

    /// Indicates a new rollback or event is available and must be processed.
    ///
    /// This does not necesserily schedule a wakeup, so the source must be polled again after this is processed.
    Interrupt {
        /// The time the information pertains to
        time: T,

        /// The type of interrupt
        interrupt: Interrupt<E>,

        /// The current finalize bound (after this interrupt is processed)
        interrupt_lower_bound: LowerBound<T>,
    },

    /// pending operation. interrupt waker will be called when progress may be made toward interrupts being resolved.
    InterruptPending,
}

impl<T, E, S> SourcePoll<T, E, Poll<S>> {
    pub fn map_state<F, U>(self, f: F) -> SourcePoll<T, E, Poll<U>>
    where
        F: FnOnce(S) -> U,
    {
        match self {
            SourcePoll::StateProgress {
                state,
                next_event_at,
                interrupt_lower_bound,
            } => SourcePoll::StateProgress {
                state: state.map(f),
                next_event_at,
                interrupt_lower_bound,
            },
            SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            } => SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            },
            SourcePoll::InterruptPending => SourcePoll::InterruptPending,
        }
    }

    pub fn remove_state<F, U>(self, f: F) -> SourcePoll<T, E, ()>
    where
        F: FnOnce(S),
    {
        match self {
            SourcePoll::StateProgress {
                state,
                next_event_at,
                interrupt_lower_bound,
            } => {
                if let Poll::Ready(s) = state {
                    f(s)
                }
                SourcePoll::StateProgress {
                    state: (),
                    next_event_at,
                    interrupt_lower_bound,
                }
            }
            SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            } => SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            },
            SourcePoll::InterruptPending => SourcePoll::InterruptPending,
        }
    }
}

impl<T, E> SourcePoll<T, E, ()> {
    pub fn set_state<F, U>(self, f: F) -> SourcePoll<T, E, Poll<U>>
    where
        F: FnOnce() -> U,
    {
        match self {
            SourcePoll::StateProgress {
                state: (),
                next_event_at,
                interrupt_lower_bound,
            } => SourcePoll::StateProgress {
                state: Poll::Ready(f()),
                next_event_at,
                interrupt_lower_bound,
            },
            SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            } => SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            },
            SourcePoll::InterruptPending => SourcePoll::InterruptPending,
        }
    }
}

impl<T, E, S> SourcePoll<T, E, S> {
    pub fn map_event<F, U>(self, f: F) -> SourcePoll<T, U, S>
    where
        F: FnOnce(&T, E) -> U,
    {
        match self {
            SourcePoll::StateProgress {
                state,
                next_event_at,
                interrupt_lower_bound,
            } => SourcePoll::StateProgress {
                state,
                next_event_at,
                interrupt_lower_bound,
            },
            SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            } => SourcePoll::Interrupt {
                interrupt: interrupt.map_event(|e| f(&time, e)),
                time,
                interrupt_lower_bound,
            },
            SourcePoll::InterruptPending => SourcePoll::InterruptPending,
        }
    }
}

impl<T: Copy, E, S> SourcePoll<T, E, S> {
    pub fn get_interrupt_lower_bound(&self) -> Option<LowerBound<T>> {
        match self {
            SourcePoll::StateProgress {
                interrupt_lower_bound,
                ..
            } => Some(*interrupt_lower_bound),
            SourcePoll::Interrupt {
                interrupt_lower_bound,
                ..
            } => Some(*interrupt_lower_bound),
            SourcePoll::InterruptPending => None,
        }
    }
}

/// A bound for describing ranges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBound<T> {
    /// a set bounded below by this includes all t.
    /// a set bounded above by this includes no t.
    Min,

    /// a set bounded below by this includes all t >= T.
    /// a set bounded above by this includes all t <= T.
    Inclusive(T),

    /// a set bounded below by this includes all t > T.
    /// a set bounded above by this includes all t < T.
    Exclusive(T),

    /// a set bounded below by this includes no t.
    /// a set bounded above by this includes all t.
    Max,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowerBound<T>(pub SourceBound<T>);

impl<T: PartialOrd> PartialOrd for LowerBound<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;

        match (&self.0, &other.0) {
            (SourceBound::Min, SourceBound::Min) => Some(Equal),
            (SourceBound::Max, SourceBound::Max) => Some(Equal),
            (SourceBound::Min, _) | (_, SourceBound::Max) => Some(Less),
            (SourceBound::Max, _) | (_, SourceBound::Min) => Some(Greater),
            (SourceBound::Inclusive(t1), SourceBound::Inclusive(t2))
            | (SourceBound::Exclusive(t1), SourceBound::Exclusive(t2)) => t1.partial_cmp(t2),
            (SourceBound::Exclusive(t1), SourceBound::Inclusive(t2)) => {
                Some(t1.partial_cmp(t2)?.then(Less))
            }
            (SourceBound::Inclusive(t1), SourceBound::Exclusive(t2)) => {
                Some(t1.partial_cmp(t2)?.then(Greater))
            }
        }
    }
}

impl<T: Ord> Ord for LowerBound<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;

        match (&self.0, &other.0) {
            (SourceBound::Min, SourceBound::Min) => Equal,
            (SourceBound::Max, SourceBound::Max) => Equal,
            (SourceBound::Min, _) | (_, SourceBound::Max) => Less,
            (SourceBound::Max, _) | (_, SourceBound::Min) => Greater,
            (SourceBound::Inclusive(t1), SourceBound::Inclusive(t2))
            | (SourceBound::Exclusive(t1), SourceBound::Exclusive(t2)) => t1.cmp(t2),
            (SourceBound::Exclusive(t1), SourceBound::Inclusive(t2)) => t1.cmp(t2).then(Less),
            (SourceBound::Inclusive(t1), SourceBound::Exclusive(t2)) => t1.cmp(t2).then(Greater),
        }
    }
}

impl<T> LowerBound<T> {
    pub fn min() -> Self {
        Self(SourceBound::Min)
    }

    pub fn max() -> Self {
        Self(SourceBound::Max)
    }

    pub fn inclusive(t: T) -> Self {
        Self(SourceBound::Inclusive(t))
    }

    pub fn exclusive(t: T) -> Self {
        Self(SourceBound::Exclusive(t))
    }
}

impl<T: Ord> LowerBound<T> {
    pub fn test(&self, value: &T) -> bool {
        match &self.0 {
            SourceBound::Min => true,
            SourceBound::Inclusive(t) => t <= value,
            SourceBound::Exclusive(t) => t < value,
            SourceBound::Max => false,
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpperBound<T>(pub SourceBound<T>);

impl<T: PartialOrd> PartialOrd for UpperBound<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::*;

        match (&self.0, &other.0) {
            (SourceBound::Min, SourceBound::Min) => Some(Equal),
            (SourceBound::Max, SourceBound::Max) => Some(Equal),
            (SourceBound::Min, _) | (_, SourceBound::Max) => Some(Less),
            (SourceBound::Max, _) | (_, SourceBound::Min) => Some(Greater),
            (SourceBound::Inclusive(t1), SourceBound::Inclusive(t2))
            | (SourceBound::Exclusive(t1), SourceBound::Exclusive(t2)) => t1.partial_cmp(t2),
            (SourceBound::Exclusive(t1), SourceBound::Inclusive(t2)) => {
                Some(t1.partial_cmp(t2)?.then(Greater))
            }
            (SourceBound::Inclusive(t1), SourceBound::Exclusive(t2)) => {
                Some(t1.partial_cmp(t2)?.then(Less))
            }
        }
    }
}

impl<T: Ord> Ord for UpperBound<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;

        match (&self.0, &other.0) {
            (SourceBound::Min, SourceBound::Min) => Equal,
            (SourceBound::Max, SourceBound::Max) => Equal,
            (SourceBound::Min, _) | (_, SourceBound::Max) => Less,
            (SourceBound::Max, _) | (_, SourceBound::Min) => Greater,
            (SourceBound::Inclusive(t1), SourceBound::Inclusive(t2))
            | (SourceBound::Exclusive(t1), SourceBound::Exclusive(t2)) => t1.cmp(t2),
            (SourceBound::Exclusive(t1), SourceBound::Inclusive(t2)) => t1.cmp(t2).then(Greater),
            (SourceBound::Inclusive(t1), SourceBound::Exclusive(t2)) => t1.cmp(t2).then(Less),
        }
    }
}

impl<T> UpperBound<T> {
    pub fn min() -> Self {
        Self(SourceBound::Min)
    }

    pub fn max() -> Self {
        Self(SourceBound::Max)
    }

    pub fn inclusive(t: T) -> Self {
        Self(SourceBound::Inclusive(t))
    }

    pub fn exclusive(t: T) -> Self {
        Self(SourceBound::Exclusive(t))
    }
}

impl<T: Ord> UpperBound<T> {
    pub fn test(&self, value: &T) -> bool {
        match &self.0 {
            SourceBound::Min => false,
            SourceBound::Inclusive(t) => t >= value,
            SourceBound::Exclusive(t) => t > value,
            SourceBound::Max => true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
/// The type of interrupt emitted from the source
pub enum Interrupt<E> {
    /// A new event is available.
    Event(E),

    /// All events at or after time T must be discarded.
    Rollback,
}

impl<E> Interrupt<E> {
    pub fn map_event<F, U>(self, f: F) -> Interrupt<U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            Interrupt::Event(e) => Interrupt::Event(f(e)),
            Interrupt::Rollback => Interrupt::Rollback,
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

impl PartialEq for SourcePollErr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SpecificError(_), Self::SpecificError(_)) => false,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

pub type TrySourcePoll<T, E, S> = Result<SourcePoll<T, E, S>, SourcePollErr>;
