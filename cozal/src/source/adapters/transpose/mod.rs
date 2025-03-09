// #![allow(dead_code)]

use archery::ArcTK;
use erased_input_source_collection::ErasedInputSourceCollection;
use hashbrown::{HashMap, HashSet};
use input_channel_reservations::InputChannelReservations;
use input_source_metadata::InputSourceMetaData;
use std::collections::BTreeSet;
use std::future::Future;
use std::num::NonZeroUsize;
use std::ops::Bound;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use steps::StepList;
use transpose_interrupt_waker::{
    ChannelItem, InnerGuard, Status, StepItem, TransposeWakerObserver,
};

// mod builder;
mod erased_input_source_collection;
mod input_channel_reservations;
mod input_source_metadata;
mod steps;
mod transpose_interrupt_waker;

// #[cfg(test)]
// mod test;

pub use builder::TransposeBuilder;

use crate::source::source_poll::{Interrupt, SourcePollErr, TrySourcePoll};
use crate::source::traits::SourceContext;
use crate::source::{Source, SourcePoll};
use crate::transposer::input_erasure::HasErasedInputExt;
use crate::transposer::step::{BoxedInput, Interpolation, StepPoll};
use crate::transposer::Transposer;

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
    input_sources: ErasedInputSourceCollection<T, InputSourceMetaData<T>>,

    // the working steps
    steps: StepList<T>,

    // the inputs not yet consumed by the steps
    input_buffer: BTreeSet<BoxedInput<'static, T, ArcTK>>,

    // uuid -> (forget, interpolation)
    interpolations: HashMap<u64, (bool, Pin<Box<Interpolation<T, ArcTK>>>)>,

    // the next uuid to assign to an interpolation
    next_interpolation_uuid: u64,

    // which input channel reservations are reserved (used for determining which new ones to reserve)
    channel_reservations: InputChannelReservations,

    // the max of all time values ever passed to any of the poll variants.
    wavefront_time: Option<T::Time>,

    // the latest time we have had advance called to.
    advance_time: Option<T::Time>,

    // if advance_final has ever been called.
    advance_final: bool,

    // if we have ever returned the Complete interrupt (or if we just decided to emit it and needs_signal was just set)
    complete: Option<T::Time>,

    // the latest value of finalize we have emitted via interrupts (or if we just decided to increase and emit it and needs_signal was just set)
    last_finalize: Option<T::Time>,

    // whether we still need to actually emit interrupts for complete or last_finalize
    needs_signal: bool,
}

impl<T: Transposer + Clone + 'static> TransposeMain<T> {
    #[cfg(debug_assertions)]
    fn assert_at_rest_structure(&self) {
        let last_step = self.steps.get_last_step();
        // the first item in the input buffer is after the last step.
        if let Some(i) = self.input_buffer.first() {
            assert!(i.get_time() > last_step.step.get_time());
        }

        if last_step.step.is_saturated() {
            assert!(last_step
                .step
                .next_scheduled_unsaturated()
                .unwrap()
                .is_none());
            assert!(self.input_buffer.is_empty());
        }
    }

    fn update_complete_and_finalize_time(&mut self) {
        let earliest_possible_interrupt = self
            .input_sources
            .earliest_possible_incoming_interrupt_time();
        let earliest_possible_output_event = self.steps.earliest_possible_event_time();
        println!("earliest_possible_interrupt: {:?}", earliest_possible_interrupt);
        println!("earliest_possible_output_event: {:?}", earliest_possible_output_event);

        let finalize_time = match (earliest_possible_interrupt, earliest_possible_output_event) {
            (None, None) => {
                if self.complete.is_none() {
                    self.complete = self.last_finalize;
                    self.needs_signal = true;

                    if let Some(t) = self.advance_time {
                        self.input_sources.advance_all_inputs(t);
                    } else {
                        self.input_sources.advance_final_all_inputs();
                    }
                }
                return;
            }
            (None, Some(t)) => Some(t),
            (Some(Some(t)), None) => Some(t),
            (Some(None), Some(t)) => Some(t),
            (Some(None), None) => None,
            (Some(Some(t1)), Some(t2)) => Some(t1.min(t2)),
        };

        if let Some(finalize_time) = finalize_time {
            if self.last_finalize < Some(finalize_time) {
                self.last_finalize = Some(finalize_time);
                self.needs_signal = true;

                if self.advance_final {
                    self.input_sources.advance_final_all_inputs();
                } else if let Some(advance_time) = self.advance_time {
                    self.input_sources.advance_all_inputs(advance_time);
                }
            }
        }
    }

    fn calculate_next_scheduled_time(&self) -> Option<T::Time> {
        let last_step = self.steps.get_last_step();
        let last_step_time = if last_step.step.is_saturated() {
            None
        } else {
            Some(last_step.step.get_time())
        };
        self.input_sources
            .iter()
            .filter_map(|(_, m)| m.next_scheduled_time())
            .chain(last_step_time)
            .min()
    }
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

    // ensure the last step structure is valid.
    // - last step is only saturated if there are no future input events or scheduled events
    // - the wakers step item points at the top step.
    fn rollback_step_cleanup(&mut self) {
        let last_step = self.main.steps.get_last_step();
        if last_step.step.is_saturated() {
            if let Some(next_step) = last_step
                .step
                .next_unsaturated(&mut self.main.input_buffer)
                .unwrap()
            {
                self.main.steps.push_step(next_step);
                self.wakers.step_item = None;
            } else {
                self.wakers.step_item = None;
            }
        } else {
            // do nothing. there is only ever one step which is not saturated, so this must have been the last one anyway.
            // our status should match
            debug_assert_eq!(
                Some(last_step.uuid),
                self.wakers.step_item.as_ref().map(|s| s.step_uuid)
            );
        }
    }

    fn clean_up_deleted_interpolations(&mut self, deleted_interpolations: HashSet<u64>) {
        let input_channels_to_release = self
            .wakers
            .channels
            .extract_if(|_, c| deleted_interpolations.contains(&c.interpolation_uuid))
            .filter_map(|(_, c)| match c.input_state_status {
                Status::Ready {
                    input_hash,
                    input_channel,
                }
                | Status::Pending {
                    input_hash,
                    input_channel,
                } => Some((input_hash, input_channel)),
                Status::None => None,
            });

        for (input_hash, input_channel) in input_channels_to_release {
            self.main
                .input_sources
                .get_input_by_hash(input_hash)
                .unwrap()
                .release_channel(input_channel);
            self.main
                .channel_reservations
                .clear_channel(input_hash, input_channel);
        }
    }

    fn remove_interpolations_before_advanced(&mut self) {
        let deleted_interpolations = if self.main.advance_final {
            self.main
                .interpolations
                .drain()
                .map(|(uuid, _)| uuid)
                .collect()
        } else if let Some(advanced_time) = self.main.advance_time {
            self.main
                .interpolations
                .extract_if(|_, (_, i)| i.get_time() < advanced_time)
                .map(|(uuid, _)| uuid)
                .collect()
        } else {
            return;
        };

        self.clean_up_deleted_interpolations(deleted_interpolations);
    }

    // ensure the interpolation structure is valid.
    // - no interpolations before the last saturated step.
    // - no waker channel associations are dangling
    // - input sources have their channels released
    // - no channel reservations are dangling
    fn rollback_interpolation_cleanup(&mut self, time: T::Time) {
        let deleted_interpolations = self
            .main
            .interpolations
            .extract_if(|_, (_, i)| i.get_time() >= time)
            .map(|(uuid, _)| uuid)
            .collect();

        self.clean_up_deleted_interpolations(deleted_interpolations);
    }

    fn remove_old_unneeded_steps(&mut self) {
        // todo!()
    }

    // this does not emit finalizes.
    // the interrupt this is given should be generated by polling the wrapper form the input_sources which tracks
    // the metadata about finalizes/advances/completions.
    fn handle_interrupt(
        &mut self,
        input_hash: u64,
        time: T::Time,
        interrupt: Interrupt<BoxedInput<'static, T, ArcTK>>,
    ) -> Option<(T::Time, Interrupt<T::OutputEvent>)> {
        let result = match interrupt {
            Interrupt::Event(e) | Interrupt::FinalizedEvent(e) => {
                let step_len_before = self.main.steps.len();

                // if the event inserts into the step list, revert the step list so we can insert it.
                let inputs = self
                    .main
                    .steps
                    .delete_at_or_after(time)
                    .into_iter()
                    .chain(Some(e));
                self.main.input_buffer.extend(inputs);
                let steps_removed = self.main.steps.len() < step_len_before;
                if steps_removed {
                    // clean up state
                    self.rollback_step_cleanup();
                    self.rollback_interpolation_cleanup(time);
                    Some((time, Interrupt::Rollback))
                } else {
                    None
                }
            }
            Interrupt::Rollback => {
                // delete input buffer items
                self.main
                    .input_buffer
                    .retain(|i| i.get_input_hash() != input_hash);

                // delete steps, pushing the inputs back into the input buffer if they are not from
                // the input source that caused the rollback.
                let inputs = self
                    .main
                    .steps
                    .delete_at_or_after(time)
                    .into_iter()
                    .filter(|i| i.get_input_hash() != input_hash);
                self.main.input_buffer.extend(inputs);

                // clean up state
                self.rollback_step_cleanup();
                self.rollback_interpolation_cleanup(time);
                Some((time, Interrupt::Rollback))
            }
            // the finalization doesn't need to be processed in this part because the input_sources records
            // the finalize times, which we read after the match.
            _ => None,
        };

        self.main.update_complete_and_finalize_time();
        self.remove_old_unneeded_steps();

        result
    }

    fn handle_new_cx(
        &mut self,
        poll_time: T::Time,
        cx: &SourceContext,
        events_only: bool,
        forget: bool,
    ) {
        // keep the interrupt waker up to date
        self.wakers.interrupt_waker = cx.interrupt_waker.clone();

        // if the wavefront time has moved forward, mark all input_sources which
        // previously returned Ready(t) where t < new_wavefront as ready to be polled again.
        self.main.wavefront_time = Some(poll_time).max(self.main.wavefront_time);

        let wavefront_time = self.main.wavefront_time.unwrap();
        for (source_hash, _, metadata) in self.main.input_sources.iter_with_hashes() {
            if let Some(t) = metadata.next_scheduled_time() {
                if t < wavefront_time {
                    if let Some(pos) = self
                        .wakers
                        .state_interrupt_pending
                        .iter()
                        .position(|&x| x == source_hash)
                    {
                        self.wakers.state_interrupt_pending.remove(pos);
                    } else if self.wakers.state_interrupt_woken.contains(&source_hash) {
                        continue;
                    }

                    self.wakers.state_interrupt_woken.push_back(source_hash);

                    // assert the items are unique.
                    debug_assert!({
                        let mut seen = HashSet::new();
                        self.wakers
                            .state_interrupt_woken
                            .iter()
                            .chain(self.wakers.state_interrupt_pending.iter())
                            .all(|item| seen.insert(item))
                    })
                }
            }
        }

        if events_only {
            return;
        }

        // delete interpolation if the previous call to this channel was something else.
        // update channel waker otherwise
        if let std::collections::hash_map::Entry::Occupied(mut channel_entry) =
            self.wakers.channels.entry(cx.channel)
        {
            if let hashbrown::hash_map::Entry::Occupied(interpolation_entry) = self
                .main
                .interpolations
                .entry(channel_entry.get().interpolation_uuid)
            {
                let (prev_forget, interpolation) = interpolation_entry.get();
                if *prev_forget != forget || interpolation.get_time() != poll_time {
                    interpolation_entry.remove();
                    match channel_entry.remove().input_state_status {
                        Status::Ready {
                            input_hash,
                            input_channel,
                        }
                        | Status::Pending {
                            input_hash,
                            input_channel,
                        } => {
                            self.main
                                .channel_reservations
                                .clear_channel(input_hash, input_channel);
                        }
                        _ => {}
                    }
                } else {
                    channel_entry.get_mut().waker = cx.channel_waker.clone();
                }
            }
        }
    }

    fn poll_inner(
        &mut self,
        poll_time: T::Time,
        cx: SourceContext,
        events_only: bool,
        forget: bool,
    ) -> TrySourcePoll<T::Time, T::OutputEvent, Option<T::OutputState>> {
        if self.main.advance_final || self.main.advance_time > Some(poll_time) {
            return Err(SourcePollErr::PollAfterAdvance);
        }

        self.handle_new_cx(poll_time, &cx, events_only, forget);

        // handle_new_cx ensures that this is Some.
        let wavefront_time = self.main.wavefront_time.unwrap();

        // check inputs for new interrupts
        loop {
            if self.main.needs_signal {
                self.main.needs_signal = false;
                if let Some(t) = self.main.complete {
                    return Ok(SourcePoll::Interrupt {
                        time: t,
                        interrupt: Interrupt::Complete,
                    });
                } else {
                    return Ok(SourcePoll::Interrupt {
                        time: self.main.last_finalize.unwrap(),
                        interrupt: Interrupt::Finalize,
                    });
                }
            }

            #[cfg(debug_assertions)]
            self.main.assert_at_rest_structure();

            // first resolve all the state interrupt woken inputs
            if let Some(input_hash) = self.wakers.state_interrupt_woken.pop_front() {
                let interrupt_waker = self.outer_wakers.get_source_interrupt_waker(input_hash);
                let mut input_source = self
                    .main
                    .input_sources
                    .get_input_by_hash(input_hash)
                    .unwrap();
                match input_source.poll_events(wavefront_time, interrupt_waker)? {
                    SourcePoll::Ready { .. } => continue,
                    SourcePoll::Interrupt { time, interrupt } => {
                        match self.handle_interrupt(input_hash, time, interrupt) {
                            Some((mapped_time, interrupt)) => {
                                break Ok(SourcePoll::Interrupt {
                                    time: mapped_time,
                                    interrupt,
                                })
                            }
                            None => continue,
                        };
                    }
                    SourcePoll::Pending => {
                        self.wakers.state_interrupt_pending.push_back(input_hash);
                        continue;
                    }
                }
            }

            if !self.wakers.state_interrupt_pending.is_empty() {
                break Ok(SourcePoll::Pending);
            }

            // start a step saturation if we need to.
            if self.wakers.step_item.is_none() {
                if let Some((step_a, step_b)) = self.main.steps.get_last_two_steps() {
                    if step_b.step.is_unsaturated() && step_b.step.get_time() <= wavefront_time {
                        step_b.step.start_saturate_clone(&step_a.step).unwrap();
                        self.wakers.step_item = Some(StepItem {
                            step_uuid: step_b.uuid,
                            step_woken: true,
                            input_state_status: Status::None,
                        });
                    }
                }
            }

            // step polling (+ input states initiated by step polls)
            if let Some(step_item) = &mut self.wakers.step_item {
                if let Status::Ready {
                    input_hash,
                    input_channel,
                } = step_item.input_state_status
                {
                    let step_source_waker = self
                        .outer_wakers
                        .get_source_step_waker(input_hash, step_item.step_uuid);
                    let mut source = self
                        .main
                        .input_sources
                        .get_input_by_hash(input_hash)
                        .unwrap();
                    let step_wrapper = self
                        .main
                        .steps
                        .get_step_wrapper_mut_by_uuid(step_item.step_uuid)
                        .unwrap();
                    let time = step_wrapper.step.get_time();

                    let context = SourceContext {
                        channel: input_channel,
                        channel_waker: step_source_waker.clone(),
                        interrupt_waker: step_source_waker,
                    };

                    match source.poll(time, context).unwrap() {
                        SourcePoll::Ready { state, .. } => {
                            match step_wrapper.step.provide_input_state(state) {
                                Ok(()) => {}
                                Err(_) => panic!(),
                            }
                            step_item.input_state_status = Status::None;
                            step_item.step_woken = true;
                        }
                        SourcePoll::Interrupt { time, interrupt } => {
                            match self.handle_interrupt(input_hash, time, interrupt) {
                                Some((mapped_time, interrupt)) => {
                                    break Ok(SourcePoll::Interrupt {
                                        time: mapped_time,
                                        interrupt,
                                    })
                                }
                                None => continue,
                            };
                        }
                        SourcePoll::Pending => break Ok(SourcePoll::Pending),
                    }
                }

                if !step_item.step_woken {
                    break Ok(SourcePoll::Pending);
                }

                let step_wrapper = self
                    .main
                    .steps
                    .get_step_wrapper_mut_by_uuid(step_item.step_uuid)
                    .unwrap();
                let waker = self.outer_wakers.get_step_waker(step_item.step_uuid);

                let poll = step_wrapper.step.poll(&waker).unwrap();
                let step_time = step_wrapper.step.get_time();
                match poll {
                    StepPoll::Ready => {
                        if let Some(next_step) = step_wrapper
                            .step
                            .next_unsaturated(&mut self.main.input_buffer)
                            .unwrap()
                        {
                            self.main.steps.push_step(next_step);
                        }
                        self.wakers.step_item = None;
                        self.main.update_complete_and_finalize_time();
                        continue;
                    }
                    StepPoll::Emitted(e) => {
                        self.main.update_complete_and_finalize_time();
                        let time = step_time;
                        let emit_finalize = match self
                            .main
                            .input_sources
                            .earliest_possible_incoming_interrupt_time()
                        {
                            None => true,
                            Some(None) => false,
                            Some(Some(t)) => t > time,
                        };

                        if emit_finalize {
                            self.main.last_finalize = Some(time);
                            self.main.needs_signal = false;
                            break Ok(SourcePoll::Interrupt {
                                time: step_time,
                                interrupt: Interrupt::FinalizedEvent(e),
                            });
                        } else {
                            break Ok(SourcePoll::Interrupt {
                                time: step_time,
                                interrupt: Interrupt::Event(e),
                            });
                        }
                    }
                    StepPoll::Pending => break Ok(SourcePoll::Pending),
                    StepPoll::StateRequested(input) => {
                        let input_hash = input.get_hash();
                        step_item.input_state_status = Status::Ready {
                            input_hash,
                            input_channel: 0,
                        };
                        continue;
                    }
                }
            }

            if events_only {
                break Ok(SourcePoll::Ready {
                    state: None,
                    next_event_at: self.main.calculate_next_scheduled_time(),
                });
            }

            let channel_item = match self.wakers.channels.entry(cx.channel) {
                std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    occupied_entry.into_mut()
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    let interpolation_uuid = self.main.next_interpolation_uuid;
                    self.main.next_interpolation_uuid += 1;
                    self.main.interpolations.insert(
                        interpolation_uuid,
                        (
                            forget,
                            Box::pin(self.main.steps.create_interpolation(poll_time)),
                        ),
                    );
                    vacant_entry.insert(ChannelItem {
                        interpolation_uuid,
                        waker: cx.channel_waker.clone(),
                        interpolation_woken: true,
                        input_state_status: Status::None,
                    })
                }
            };

            // step polling (+ input states initiated by step polls)
            if let Status::Ready {
                input_hash,
                input_channel,
            } = channel_item.input_state_status
            {
                let source_channel_waker = self
                    .outer_wakers
                    .get_source_channel_waker(input_hash, channel_item.interpolation_uuid);
                let mut source = self
                    .main
                    .input_sources
                    .get_input_by_hash(input_hash)
                    .unwrap();
                let source_interrupt_waker =
                    self.outer_wakers.get_source_interrupt_waker(input_hash);
                let (_, interpolation) = self
                    .main
                    .interpolations
                    .get_mut(&channel_item.interpolation_uuid)
                    .unwrap();
                let time = interpolation.get_time();

                let context = SourceContext {
                    channel: input_channel,
                    channel_waker: source_channel_waker,
                    interrupt_waker: source_interrupt_waker,
                };

                let poll = if forget {
                    source.poll_forget(time, context)
                } else {
                    source.poll(time, context)
                };

                match poll.unwrap() {
                    SourcePoll::Ready { state, .. } => {
                        match interpolation
                            .as_mut()
                            .get_input_state_manager()
                            .provide_input_state(state)
                        {
                            Ok(()) => {}
                            Err(_) => panic!(),
                        }
                        self.main
                            .channel_reservations
                            .clear_channel(input_hash, input_channel);
                        channel_item.input_state_status = Status::None;
                        channel_item.interpolation_woken = true;
                    }
                    SourcePoll::Interrupt { time, interrupt } => {
                        match self.handle_interrupt(input_hash, time, interrupt) {
                            Some((mapped_time, interrupt)) => {
                                break Ok(SourcePoll::Interrupt {
                                    time: mapped_time,
                                    interrupt,
                                })
                            }
                            None => continue,
                        };
                    }
                    SourcePoll::Pending => break Ok(SourcePoll::Pending),
                }
            }

            if !channel_item.interpolation_woken {
                break Ok(SourcePoll::Pending);
            }

            let mut interpolation = self
                .main
                .interpolations
                .get_mut(&channel_item.interpolation_uuid)
                .unwrap()
                .1
                .as_mut();
            let waker = self
                .outer_wakers
                .get_interpolation_waker(channel_item.interpolation_uuid);
            match interpolation
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
            {
                Poll::Ready(state) => {
                    self.main
                        .interpolations
                        .remove(&channel_item.interpolation_uuid);
                    self.wakers.channels.remove(&cx.channel);
                    break Ok(SourcePoll::Ready {
                        state: Some(state),
                        next_event_at: self.main.calculate_next_scheduled_time(),
                    });
                }
                Poll::Pending => {
                    match interpolation.get_input_state_manager().try_accept_request() {
                        Some(input) => {
                            let input_hash = input.get_hash();
                            let entry = self
                                .main
                                .channel_reservations
                                .get_first_available_channel(input_hash);
                            let input_channel = entry.get().input_channel;
                            entry.insert();
                            channel_item.input_state_status = Status::Ready {
                                input_hash,
                                input_channel,
                            };
                        }
                        None => break Ok(SourcePoll::Pending),
                    }
                }
            }
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
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        let mut locked = TransposeLocked::from_transpose(self);
        Ok(locked
            .poll_inner(time, cx, false, false)?
            .map_state(|state| state.unwrap()))
    }

    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        let mut locked = TransposeLocked::from_transpose(self);
        Ok(locked
            .poll_inner(time, cx, false, true)?
            .map_state(|state| state.unwrap()))
    }

    fn poll_events(
        &mut self,
        time: Self::Time,
        interrupt_waker: std::task::Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        let mut locked = TransposeLocked::from_transpose(self);
        let cx = SourceContext {
            channel: 0,
            channel_waker: Waker::noop().clone(),
            interrupt_waker,
        };
        Ok(locked
            .poll_inner(time, cx, true, true)?
            .map_state(|state| debug_assert!(state.is_none())))
    }

    fn release_channel(&mut self, channel: usize) {
        let mut locked = TransposeLocked::from_transpose(self);
        if let Some(channel_item) = locked.wakers.channels.remove(&channel) {
            locked
                .main
                .interpolations
                .remove(&channel_item.interpolation_uuid);
            if let Status::Ready {
                input_hash,
                input_channel,
            }
            | Status::Pending {
                input_hash,
                input_channel,
            } = channel_item.input_state_status
            {
                locked
                    .main
                    .channel_reservations
                    .clear_channel(input_hash, input_channel);
                locked
                    .main
                    .input_sources
                    .get_input_by_hash(input_hash)
                    .unwrap()
                    .release_channel(input_channel);
            }
        }
    }

    fn advance(&mut self, lower_bound: Bound<Self::Time>) {
        let mut locked = TransposeLocked::from_transpose(self);
        locked.main.advance_time = locked.main.advance_time.max(Some(time));
        locked.main.update_complete_and_finalize_time();
        locked.remove_interpolations_before_advanced();
        locked.remove_old_unneeded_steps();
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        self.main
            .input_sources
            .iter()
            .map(|(s, _)| s.max_channel())
            .min()
            .unwrap_or(NonZeroUsize::MIN)
    }
}
