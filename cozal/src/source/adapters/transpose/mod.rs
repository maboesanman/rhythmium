#![allow(dead_code)]

use archery::ArcTK;
use erased_input_source_collection::ErasedInputSourceCollection;
use hashbrown::HashMap;
use input_channel_reservations::InputChannelReservations;
use input_source_metadata::InputSourceMetaData;
use parking_lot::MutexGuard;
use std::collections::BTreeSet;
use std::future::Future;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use steps::StepList;
use transpose_interrupt_waker::{
    ChannelItem, Status, TransposeInterruptWakerInner, TransposeWakerObserver,
};

mod channel_status;
mod erased_input_source_collection;
mod input_channel_reservations;
mod input_source_metadata;
mod steps;
mod transpose_interrupt_waker;

use crate::source::source_poll::{Interrupt, TrySourcePoll};
use crate::source::traits::SourceContext;
use crate::source::{Source, SourcePoll};
use crate::transposer::input_erasure::HasErasedInputExt;
use crate::transposer::step::{BoxedInput, Interpolation, StepPoll};
use crate::transposer::Transposer;

pub struct Transpose<T: Transposer + 'static> {
    // The sources we are transposing.
    input_sources: ErasedInputSourceCollection<T, InputSourceMetaData<T>>,

    // the working steps
    steps: StepList<T>,

    // the inputs not yet consumed by the steps
    input_buffer: BTreeSet<BoxedInput<'static, T, ArcTK>>,

    // by uuid
    interpolations: HashMap<u64, Pin<Box<Interpolation<T, ArcTK>>>>,
    next_interpolation_uuid: u64,

    wavefront_time: T::Time,
    advance_time: T::Time,

    channel_reservations: InputChannelReservations,

    // input_channel_statuses: InputChannelStatuses<T>,
    wakers: TransposeWakerObserver,

    complete: bool,
    last_finalize: T::Time,
}

struct TransposeLocked<'a, T: Transposer + 'static> {
    // The sources we are transposing.
    input_sources: &'a mut ErasedInputSourceCollection<T, InputSourceMetaData<T>>,

    // the working steps
    steps: &'a mut StepList<T>,

    // the inputs not yet consumed by the steps
    input_buffer: &'a mut BTreeSet<BoxedInput<'static, T, ArcTK>>,

    interpolations: &'a mut HashMap<u64, Pin<Box<Interpolation<T, ArcTK>>>>,
    next_interpolation_uuid: &'a mut u64,

    wavefront_time: &'a mut T::Time,
    advance_time: &'a mut T::Time,

    channel_reservations: &'a mut InputChannelReservations,

    // input_channel_statuses: InputChannelStatuses<T>,
    wakers: MutexGuard<'a, TransposeInterruptWakerInner>,

    // trying to lock this will always deadlock
    outer_wakers: &'a TransposeWakerObserver,

    complete: &'a mut bool,
    last_finalize: &'a mut T::Time,
}

impl<T: Transposer + 'static> Drop for TransposeLocked<'_, T> {
    fn drop(&mut self) {
        // clean up, if i do any mutex swapping.
    }
}

impl<T: Transposer + Clone + 'static> TransposeLocked<'_, T> {
    fn from_transpose(transpose: &mut Transpose<T>) -> TransposeLocked<'_, T> {
        let wakers = transpose.wakers.lock();
        let outer_wakers = &transpose.wakers;

        TransposeLocked {
            input_sources: &mut transpose.input_sources,
            steps: &mut transpose.steps,
            input_buffer: &mut transpose.input_buffer,
            interpolations: &mut transpose.interpolations,
            next_interpolation_uuid: &mut transpose.next_interpolation_uuid,
            wavefront_time: &mut transpose.wavefront_time,
            advance_time: &mut transpose.advance_time,
            channel_reservations: &mut transpose.channel_reservations,
            wakers,
            outer_wakers,
            complete: &mut transpose.complete,
            last_finalize: &mut transpose.last_finalize,
        }
    }

    fn handle_interrupt(
        &mut self,
        input_hash: u64,
        time: T::Time,
        interrupt: Interrupt<BoxedInput<'static, T, ArcTK>>,
    ) -> Option<(T::Time, Interrupt<T::OutputEvent>)> {
        match interrupt {
            Interrupt::Event(e) => {
                let inputs = self.steps.delete_at_or_after(time);
                self.input_buffer.extend(inputs);
                // figure out if i need to issue a rollback
                todo!();
                None
            }
            Interrupt::FinalizedEvent(e) => {
                let inputs = self.steps.delete_at_or_after(time);
                self.input_buffer.extend(inputs);
                self.input_sources
                    .handle_advance_and_finalize(*self.advance_time);
                // figure out if i need to issue a rollback
                todo!();
                None
            }
            Interrupt::Rollback => {
                // delete input buffer items
                self.input_buffer.retain(|i| i.get_input_hash() != input_hash);

                // delete steps, pushing the inputs back into the input buffer if they are not from
                // the input source that caused the rollback.
                let inputs = self
                    .steps
                    .delete_at_or_after(time)
                    .into_iter()
                    .filter(|i| i.get_input_hash() != input_hash);
                self.input_buffer.extend(inputs);

                // delete interpolations that are now invalid.
                self.interpolations.retain(|_, i| i.get_time() < time);

                // fix the general structure of the transpose (in particular the wakers)
                todo!()
            }
            Interrupt::Finalize => {
                self.input_sources
                    .handle_advance_and_finalize(*self.advance_time);
                // match self.calculate_finalize_time() {
                //     Some(Some(finalize_time)) => {
                //         *self.last_finalize = finalize_time;
                //         Some(Interrupt::Finalize)
                //     }
                //     Some(None) => {
                //         *self.complete = true;
                //         Some(Interrupt::Complete)
                //     }
                //     None => None,
                // }
                todo!()
            }
            Interrupt::Complete => {
                self.input_sources
                    .handle_advance_and_finalize(*self.advance_time);
                todo!()
            }
        }
    }

    fn handle_new_cx(&mut self, poll_time: T::Time, cx: &SourceContext) {
        *self.wavefront_time = poll_time.max(*self.wavefront_time);
        for (source_hash, _, metadata) in self.input_sources.iter_with_hashes() {
            if let Some(t) = metadata.next_scheduled_time() {
                if t < *self.wavefront_time {
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
                }
            }
        }
        // delete interpolation if the previous call to this channel was something else.
        todo!()
    }

    fn calculate_next_scheduled_time(&self) -> Option<T::Time> {
        let last_step = self.steps.get_last_step();
        let last_step_time = if last_step.is_saturated() {
            None
        } else {
            Some(last_step.get_time())
        };
        self.input_sources
            .iter()
            .filter_map(|(_, m)| m.next_scheduled_time())
            .chain(last_step_time)
            .min()
    }

    /// None here means there will never be an event ever again.
    /// Some(None) means we have yet to finalize anything. sort of the opposite.
    fn calculate_finalize_time(&self) -> Option<Option<T::Time>> {
        if *self.complete {
            return None;
        }
        let source_finalize_times = self
            .input_sources
            .iter()
            .filter(|(_, m)| !m.complete())
            .map(|(_, m)| m.finalized_time());
        let step_saturating_time = self.steps.get_first_possible_event_emit_time().map(Some);
        let input_buffer_first_time = self.input_buffer.first().map(|i| i.get_time()).map(Some);

        source_finalize_times
            .chain(step_saturating_time)
            .chain(input_buffer_first_time)
            .min()
    }

    fn poll_inner(
        &mut self,
        poll_time: T::Time,
        cx: SourceContext,
        events_only: bool,
        forget: bool,
    ) -> TrySourcePoll<T::Time, T::OutputEvent, Option<T::OutputState>> {
        self.handle_new_cx(poll_time, &cx);
        // check inputs for new interrupts
        loop {
            // first resolve all the state interrupt woken inputs
            if let Some(input_hash) = self.wakers.state_interrupt_woken.pop_front() {
                let interrupt_waker = self.outer_wakers.get_source_interrupt_waker(input_hash);
                let mut input_source = self.input_sources.get_input_by_hash(input_hash).unwrap();
                match input_source.poll_events(*self.wavefront_time, interrupt_waker)? {
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
                    let mut source = self.input_sources.get_input_by_hash(input_hash).unwrap();
                    let step_wrapper = self
                        .steps
                        .get_step_wrapper_mut_by_uuid(step_item.step_uuid)
                        .unwrap();
                    let time = step_wrapper.step.get_time();

                    let context = SourceContext {
                        channel: input_channel,
                        channel_waker: step_source_waker.clone(),
                        interrupt_waker: step_source_waker,
                    };

                    match source.poll(time, context) {
                        Ok(SourcePoll::Ready { state, .. }) => {
                            match step_wrapper.step.provide_input_state(state) {
                                Ok(()) => {}
                                Err(_) => panic!(),
                            }
                            step_item.input_state_status = Status::None;
                            step_item.step_woken = true;
                        }
                        Ok(SourcePoll::Interrupt { time, interrupt }) => {
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
                        Ok(SourcePoll::Pending) => break Ok(SourcePoll::Pending),
                        Err(_) => panic!(),
                    }
                }

                if !step_item.step_woken {
                    break Ok(SourcePoll::Pending);
                }

                let step_wrapper = self
                    .steps
                    .get_step_wrapper_mut_by_uuid(step_item.step_uuid)
                    .unwrap();
                let waker = self.outer_wakers.get_step_waker(step_item.step_uuid);
                match step_wrapper.step.poll(&waker) {
                    Ok(StepPoll::Ready) => {
                        if let Some(next_step) = step_wrapper
                            .step
                            .next_unsaturated(self.input_buffer)
                            .unwrap()
                        {
                            let next_step = self.steps.push_step(next_step);
                            if next_step.step.get_time() <= *self.wavefront_time {
                                step_item.step_uuid = next_step.uuid;
                                step_item.step_woken = true;
                                continue;
                            }
                        }
                        self.wakers.step_item = None;
                    }
                    Ok(StepPoll::Emitted(e)) => {
                        break Ok(SourcePoll::Interrupt {
                            time: step_wrapper.step.get_time(),
                            interrupt: Interrupt::Event(e),
                        })
                    }
                    Ok(StepPoll::Pending) => break Ok(SourcePoll::Pending),
                    Ok(StepPoll::StateRequested(input)) => {
                        let input_hash = input.get_hash();
                        step_item.input_state_status = Status::Ready {
                            input_hash,
                            input_channel: 0,
                        };
                        continue;
                    }
                    Err(_) => panic!(),
                }
            }

            if events_only {
                break Ok(SourcePoll::Ready {
                    state: None,
                    next_event_at: self.calculate_next_scheduled_time(),
                });
            }

            let channel_item = match self.wakers.channels.entry(cx.channel) {
                std::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    occupied_entry.into_mut()
                }
                std::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    let interpolation_uuid = *self.next_interpolation_uuid;
                    *self.next_interpolation_uuid += 1;
                    self.interpolations.insert(
                        interpolation_uuid,
                        Box::pin(self.steps.create_interpolation(poll_time)),
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
                let mut source = self.input_sources.get_input_by_hash(input_hash).unwrap();
                let source_interrupt_waker =
                    self.outer_wakers.get_source_interrupt_waker(input_hash);
                let interpolation = self
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

                match poll {
                    Ok(SourcePoll::Ready { state, .. }) => {
                        match interpolation
                            .as_mut()
                            .get_input_state_manager()
                            .provide_input_state(state)
                        {
                            Ok(()) => {}
                            Err(_) => panic!(),
                        }
                        self.channel_reservations
                            .clear_channel(input_hash, input_channel);
                        channel_item.input_state_status = Status::None;
                        channel_item.interpolation_woken = true;
                    }
                    Ok(SourcePoll::Interrupt { time, interrupt }) => {
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
                    Ok(SourcePoll::Pending) => break Ok(SourcePoll::Pending),
                    Err(_) => panic!(),
                }
            }

            if !channel_item.interpolation_woken {
                break Ok(SourcePoll::Pending);
            }

            let mut interpolation = self
                .interpolations
                .get_mut(&channel_item.interpolation_uuid)
                .unwrap()
                .as_mut();
            let waker = self
                .outer_wakers
                .get_interpolation_waker(channel_item.interpolation_uuid);
            match interpolation
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
            {
                Poll::Ready(state) => {
                    self.interpolations.remove(&channel_item.interpolation_uuid);
                    self.wakers.channels.remove(&cx.channel);
                    break Ok(SourcePoll::Ready {
                        state: Some(state),
                        next_event_at: self.calculate_next_scheduled_time(),
                    });
                }
                Poll::Pending => {
                    match interpolation.get_input_state_manager().try_accept_request() {
                        Some(input) => {
                            let input_hash = input.get_hash();
                            let entry = self
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
            interrupt_waker: interrupt_waker,
        };
        Ok(locked
            .poll_inner(time, cx, true, true)?
            .map_state(|state| debug_assert!(state.is_none())))
    }

    fn release_channel(&mut self, channel: usize) {
        let mut locked = TransposeLocked::from_transpose(self);
        if let Some(channel_item) = locked.wakers.channels.remove(&channel) {
            locked
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
                    .channel_reservations
                    .clear_channel(input_hash, input_channel);
                locked
                    .input_sources
                    .get_input_by_hash(input_hash)
                    .unwrap()
                    .release_channel(input_channel);
            }
        }
    }

    fn advance(&mut self, time: Self::Time) {
        self.advance_time = self.advance_time.max(time);
        self.input_sources
            .handle_advance_and_finalize(self.advance_time);
    }

    fn advance_final(&mut self) {
        todo!();
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
        self.input_sources
            .iter()
            .map(|(s, _)| s.max_channel())
            .min()
            .unwrap_or(NonZeroUsize::MIN)
    }
}
