// #![allow(dead_code)]

use archery::ArcTK;
use erased_input_source_collection::ErasedInputSourceCollection;
use futures::lock;
use hashbrown::{HashMap, HashSet};
use input_channel_reservations::InputChannelReservations;
use input_source_collection::{AggregateSourcePoll, InputSourceCollection};
use input_source_metadata::InputSourceMetaData;
use working_timeline_slice::{WorkingTimelineSlice, WorkingTimelineSlicePoll};
use std::collections::BTreeSet;
use std::future::Future;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use transpose_interrupt_waker::{
    FutureStatus, InnerGuard, InputStateStatus, StepStatus, TransposeWakerObserver
};

mod builder;
mod erased_input_source_collection;
mod input_channel_reservations;
mod input_source_metadata;
mod transpose_interrupt_waker;
mod working_timeline_slice;
mod input_source_collection;

#[cfg(test)]
mod test;

pub use builder::TransposeBuilder;

use crate::source::source_poll::{Interrupt, LowerBound, SourcePollErr, TrySourcePoll, UpperBound};
use crate::source::traits::SourceContext;
use crate::source::{Source, SourcePoll};
use crate::transposer::Transposer;
use crate::transposer::input_erasure::HasErasedInputExt;
use crate::transposer::step::{BoxedInput, Interpolation, PossiblyInitStep, StepPoll};

pub struct Transpose<T: Transposer + 'static> {
    // most of the fields
    main: TransposeMain<T>,

    // input_channel_statuses: InputChannelStatuses<T>,
    wakers: TransposeWakerObserver,
}

struct TransposeLocked<'a, T: Transposer + 'static> {
    // most of the fields
    main: &'a mut TransposeMain<T>,

    // a reference to the already locked observer
    outer_wakers: &'a TransposeWakerObserver,

    // the inner state of the waker
    wakers: InnerGuard,
}

struct TransposeMain<T: Transposer + 'static> {
    // The sources we are transposing.
    pub input_sources: InputSourceCollection<T>,

    // the working steps and buffered inputs
    pub working_timeline_slice: WorkingTimelineSlice<T>,

    // // uuid -> (forget, interpolation)
    // interpolations: HashMap<u64, (bool, Pin<Box<Interpolation<T, ArcTK>>>)>,

    // // the next uuid to assign to an interpolation
    // next_interpolation_uuid: u64,

    // // which input channel reservations are reserved (used for determining which new ones to reserve)
    // channel_reservations: InputChannelReservations,

    // // the max of all time values ever passed to any of the poll variants.
    // advance_upper_bound: UpperBound<T::Time>,

    // // the latest time we have had advance called to.
    // advance_lower_bound: LowerBound<T::Time>,

    // last_emitted_finalize: LowerBound<T::Time>,

    // returned_state_times: BTreeSet<T::Time>,
}

impl<T: Transposer + Clone + 'static> TransposeLocked<'_, T> {
    fn from_transpose(transpose: &mut Transpose<T>) -> TransposeLocked<'_, T> {
        let wakers = transpose.wakers.lock();
        let outer_wakers = &transpose.wakers;

        TransposeLocked {
            main: &mut transpose.main,
            wakers,
            outer_wakers,
        }
    }
}

impl<T: Transposer + Clone + 'static> Source for Transpose<T> {
    type Time = T::Time;

    type Event = T::OutputEvent;

    type State = T::OutputState;

    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>> {
        todo!()
    }

    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Poll<Self::State>> {
        todo!()
    }

    fn poll_interrupts(
        &mut self,
        interrupt_waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        let mut locked = TransposeLocked::from_transpose(self);
        locked.wakers.interrupt_waker = interrupt_waker;

        let (next_input_event_at, inputs_interrupt_lower_bound) = 'input_interrupts: loop {
            match locked.main.input_sources.poll_aggregate_interrupts(&mut locked.wakers, |source_hash| {
                locked.outer_wakers.get_waker_for_input_poll_interrupt(source_hash)
            }) {
                AggregateSourcePoll::StateProgress { next_event_at, interrupt_lower_bound } => {
                    locked.main.working_timeline_slice.advance_interrupt_lower_bound(interrupt_lower_bound);
                    break 'input_interrupts (next_event_at, interrupt_lower_bound);
                },
                AggregateSourcePoll::Interrupt { input_hash, time, interrupt, interrupt_lower_bound } => {
                    locked.main.working_timeline_slice.advance_interrupt_lower_bound(interrupt_lower_bound);
                    if let Some(time) = locked.main.working_timeline_slice.handle_interrupt(input_hash, time, interrupt) {
                        let interrupt_lower_bound = interrupt_lower_bound.min(locked.main.working_timeline_slice.tentative_state_and_event_lower_bound());
                        return TrySourcePoll::Ok(SourcePoll::Interrupt { time, interrupt: Interrupt::Rollback, interrupt_lower_bound })
                    }
                }
                AggregateSourcePoll::InterruptPending => {
                    return TrySourcePoll::Ok(SourcePoll::InterruptPending);
                }
            }
        };

        let next_step_at = 'steps: loop {
            // check the current status, and poll the input if needed.
            if let Some(step_status) = &mut locked.wakers.step_interrupt {
                if let transpose_interrupt_waker::InputStateStatus::Woken { input_hash, input_channel } = step_status.input_state_status {
                    let time = locked.main.working_timeline_slice.get_time(step_status.step_uuid).unwrap();
                    let cx = locked.outer_wakers.get_context_for_input_poll_from_step(input_hash, step_status.step_uuid);
                    match locked.main.input_sources.poll_single(input_hash, time, cx, false) {
                        SourcePoll::StateProgress { state, next_event_at, interrupt_lower_bound } => {
                            locked.main.working_timeline_slice.advance_interrupt_lower_bound(interrupt_lower_bound);
                            if let Poll::Ready(state) = state {
                                if let Err(_) = locked.main.working_timeline_slice.provide_input_state(step_status.step_uuid, state) {
                                    panic!()
                                }
                                step_status.input_state_status = InputStateStatus::None;
                                step_status.step_saturation_future_status = FutureStatus::Woken;
                            }
                        },
                        SourcePoll::Interrupt { time, interrupt, interrupt_lower_bound } => {
                            locked.main.working_timeline_slice.advance_interrupt_lower_bound(interrupt_lower_bound);
                            if let Some(time) = locked.main.working_timeline_slice.handle_interrupt(input_hash, time, interrupt) {
                                let interrupt_lower_bound = interrupt_lower_bound.min(locked.main.working_timeline_slice.tentative_state_and_event_lower_bound());
                                return TrySourcePoll::Ok(SourcePoll::Interrupt { time, interrupt: Interrupt::Rollback, interrupt_lower_bound })
                            }
                        },
                        SourcePoll::InterruptPending => {},
                    }
                }

                if step_status.step_saturation_future_status == FutureStatus::Pending {
                    return Ok(SourcePoll::InterruptPending);
                }
            }

            locked.wakers.step_interrupt = None;

            match locked.main.working_timeline_slice.poll(|step_uuid| {
                locked.outer_wakers.get_waker_for_future_poll_from_step(step_uuid)
            }) {
                WorkingTimelineSlicePoll::Emitted { time, event } => {
                    let interrupt_lower_bound = inputs_interrupt_lower_bound.min(locked.main.working_timeline_slice.tentative_state_and_event_lower_bound());
                    return TrySourcePoll::Ok(SourcePoll::Interrupt { time, interrupt: Interrupt::Event(event), interrupt_lower_bound })
                },
                WorkingTimelineSlicePoll::StateRequested { time, input, step_uuid } => todo!(),
                WorkingTimelineSlicePoll::Ready { next_time } => {
                    break 'steps next_time;
                },
                WorkingTimelineSlicePoll::Pending { step_uuid } => {
                    locked.wakers.step_interrupt = Some(StepStatus {
                        step_uuid,
                        step_saturation_future_status: transpose_interrupt_waker::FutureStatus::Pending,
                        input_state_status: transpose_interrupt_waker::InputStateStatus::None,
                    });
                    return TrySourcePoll::Ok(SourcePoll::InterruptPending);
                }
            }
        };

        let next_event_at = match (next_input_event_at, next_step_at) {
            (None, None) => None,
            (None, Some(t)) => Some(t),
            (Some(t), None) => Some(t),
            (Some(t1), Some(t2)) => Some(t1.min(t2)),
        };
        let interrupt_lower_bound = inputs_interrupt_lower_bound.min(locked.main.working_timeline_slice.tentative_state_and_event_lower_bound());
        TrySourcePoll::Ok(SourcePoll::StateProgress { state: (), next_event_at, interrupt_lower_bound })
    }

    fn release_channel(&mut self, channel: usize) {
        todo!()
    }

    fn advance_poll_lower_bound(
        &mut self,
        poll_lower_bound: LowerBound<Self::Time>,
    ) {
        let locked = TransposeLocked::from_transpose(self);
        locked.main.input_sources.advance_poll_lower_bound(poll_lower_bound);
        locked.main.working_timeline_slice.advance_poll_lower_bound(poll_lower_bound);
    }

    fn advance_interrupt_upper_bound(
        &mut self,
        interrupt_upper_bound: UpperBound<Self::Time>,
        interrupt_waker: Waker,
    ) {
        let mut locked = TransposeLocked::from_transpose(self);
        locked.wakers.interrupt_waker = interrupt_waker;
        locked.main.input_sources.advance_interrupt_upper_bound(interrupt_upper_bound, |source_hash| {
            locked.outer_wakers.get_waker_for_input_poll_interrupt(source_hash)
        });
        locked.main.working_timeline_slice.advance_interrupt_upper_bound(interrupt_upper_bound);
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        self.main
            .input_sources
            .inputs
            .iter()
            .map(|(s, _)| s.max_channel())
            .min()
            .unwrap_or(NonZeroUsize::MIN).saturating_add(1)
    }
}
