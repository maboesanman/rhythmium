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
    InnerGuard, TransposeWakerObserver
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

//     // ensure the last step structure is valid.
//     // - last step is only saturated if there are no future input events or scheduled events
//     // - the wakers step item points at the top step.
//     fn rollback_step_cleanup(&mut self) {
//         let last_step = self.main.steps.get_last_step().unwrap();
//         if last_step.step.is_saturated() {
//             if let Some(next_step) = last_step
//                 .step
//                 .next_unsaturated(&mut self.main.input_buffer)
//                 .unwrap()
//             {
//                 self.main.steps.push_step(next_step);
//                 self.wakers.step_item = None;
//             } else {
//                 self.wakers.step_item = None;
//             }
//         } else {
//             // do nothing. there is only ever one step which is not saturated, so this must have been the last one anyway.
//             // our status should match
//             debug_assert_eq!(
//                 Some(last_step.uuid),
//                 self.wakers.step_item.as_ref().map(|s| s.step_uuid)
//             );
//         }
//     }

//     fn clean_up_deleted_interpolations(&mut self, deleted_interpolations: HashSet<u64>) {
//         let input_channels_to_release = self
//             .wakers
//             .channels
//             .extract_if(|_, c| deleted_interpolations.contains(&c.interpolation_uuid))
//             .filter_map(|(_, c)| match c.input_state_status {
//                 Status::Woken {
//                     input_hash,
//                     input_channel,
//                 }
//                 | Status::Pending {
//                     input_hash,
//                     input_channel,
//                 } => Some((input_hash, input_channel)),
//                 Status::None => None,
//             });

//         for (input_hash, input_channel) in input_channels_to_release {
//             self.main
//                 .input_sources
//                 .get_input_by_hash(input_hash)
//                 .unwrap()
//                 .release_channel(input_channel);
//             self.main
//                 .channel_reservations
//                 .clear_channel(input_hash, input_channel);
//         }
//     }

//     fn remove_interpolations_before_advanced(&mut self) {
//         let deleted_interpolations = self
//             .main
//             .interpolations
//             .extract_if(|_, (_, i)| !self.main.advance_lower_bound.test(&i.get_time()))
//             .map(|(uuid, _)| uuid)
//             .collect();

//         self.clean_up_deleted_interpolations(deleted_interpolations);
//     }

//     // ensure the interpolation structure is valid.
//     // - no interpolations before the last saturated step.
//     // - no waker channel associations are dangling
//     // - input sources have their channels released
//     // - no channel reservations are dangling
//     fn rollback_interpolation_cleanup(&mut self, time: T::Time) {
//         let deleted_interpolations = self
//             .main
//             .interpolations
//             .extract_if(|_, (_, i)| i.get_time() >= time)
//             .map(|(uuid, _)| uuid)
//             .collect();

//         self.clean_up_deleted_interpolations(deleted_interpolations);
//     }

//     fn remove_old_unneeded_steps(&mut self) {
//         // todo!()
//     }

//     // process the given interrupt, produced by the specified input hash.
//     // returns None if no rollback is needed, or Some(t) if a rollback is needed at time t.
//     fn handle_interrupt(
//         &mut self,
//         input_hash: u64,
//         time: T::Time,
//         interrupt: Interrupt<BoxedInput<'static, T, ArcTK>>,
//     ) -> Option<T::Time> {
//         // if the event inserts into the step list, revert the step list so we can insert it.
//         let deleted_steps = self
//             .main
//             .steps
//             .delete_outside_lower_bound(LowerBound::inclusive(time));

//         let invalidated_state_times = self.main.returned_state_times.split_off(&time);

//         match interrupt {
//             Interrupt::Event(event) => {
//                 let invalidated_event_times = deleted_steps.into_iter().filter_map(|step| {
//                     let event_emitted_time = step.has_produced_events().then(|| step.get_time());
//                     self.main.input_buffer.extend(step.drain_inputs());
//                     event_emitted_time
//                 });

//                 let rollback_time = invalidated_event_times.chain(invalidated_state_times).min();
//                 self.main.input_buffer.insert(event);
//                 rollback_time
//             }
//             Interrupt::Rollback => {
//                 let invalidated_event_times = deleted_steps.into_iter().filter_map(|step| {
//                     let event_emitted_time = step.has_produced_events().then(|| step.get_time());
//                     self.main.input_buffer.extend(
//                         step.drain_inputs()
//                             .into_iter()
//                             .filter(|i| i.get_input_hash() != input_hash),
//                     );
//                     event_emitted_time
//                 });

//                 let rollback_time = invalidated_event_times.chain(invalidated_state_times).min();
//                 // delete all input buffer items
//                 self.main.input_buffer.retain(|i| {
//                     let from_this_input_hash = i.get_input_hash() == input_hash;
//                     let after_rollback = i.get_time() >= time;
//                     !(from_this_input_hash && after_rollback)
//                 });
//                 rollback_time
//             }
//         }
//     }

//     fn handle_new_cx_interrupts(&mut self, interrupt_waker: &Waker) {
//         self.wakers.interrupt_waker = interrupt_waker.clone()
//     }

//     fn handle_new_cx(&mut self, poll_time: T::Time, cx: &SourceContext, forget: bool) {
//         // keep the interrupt waker up to date
//         self.handle_new_cx_interrupts(&cx.interrupt_waker);

//         // if the wavefront time has moved forward, mark all input_sources which
//         // previously returned Ready(t) where t < new_wavefront as ready to be polled again.
//         self.main.advance_upper_bound = self
//             .main
//             .advance_upper_bound
//             .max(UpperBound::inclusive(poll_time));

//         // delete interpolation if the previous call to this channel was something else.
//         // update channel waker otherwise
//         if let std::collections::hash_map::Entry::Occupied(mut channel_entry) =
//             self.wakers.channels.entry(cx.channel)
//         {
//             if let hashbrown::hash_map::Entry::Occupied(interpolation_entry) = self
//                 .main
//                 .interpolations
//                 .entry(channel_entry.get().interpolation_uuid)
//             {
//                 let (prev_forget, interpolation) = interpolation_entry.get();
//                 if *prev_forget != forget || interpolation.get_time() != poll_time {
//                     interpolation_entry.remove();
//                     match channel_entry.remove().input_state_status {
//                         Status::Woken {
//                             input_hash,
//                             input_channel,
//                         }
//                         | Status::Pending {
//                             input_hash,
//                             input_channel,
//                         } => {
//                             self.main
//                                 .channel_reservations
//                                 .clear_channel(input_hash, input_channel);
//                         }
//                         _ => {}
//                     }
//                 } else {
//                     channel_entry.get_mut().waker = cx.channel_waker.clone();
//                 }
//             }
//         }
//     }

//     fn poll_interrupts_inner(
//         &mut self,
//         interrupt_waker: &Waker,
//     ) -> TrySourcePoll<T::Time, T::OutputEvent, ()> {
//         todo!()
//         // self.wakers.interrupt_waker = interrupt_waker.clone();
//         // loop {
//         //     // handle the first woken interrupt. this will return to the top of the loop
//         //     // repeatedly unless it returns an interrupt or there are no more items in state_interrupt_woken.
//         //     if let Some(input_hash) = self.wakers.state_interrupt_woken.pop_front() {
//         //         let interrupt_waker = self.outer_wakers.get_source_interrupt_waker(input_hash);
//         //         let mut input_source = self
//         //             .main
//         //             .input_sources
//         //             .get_input_by_hash(input_hash)
//         //             .unwrap();
//         //         match input_source.poll_interrupts(interrupt_waker)? {
//         //             SourcePoll::StateProgress { .. } => continue,
//         //             SourcePoll::Interrupt {
//         //                 time, interrupt, ..
//         //             } => {
//         //                 self.wakers.state_interrupt_woken.push_back(input_hash);
//         //                 match self.handle_interrupt(input_hash, time, interrupt) {
//         //                     Some(mapped_time) => {
//         //                         break Ok(SourcePoll::Interrupt {
//         //                             time: mapped_time,
//         //                             interrupt: Interrupt::Rollback,
//         //                             interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//         //                         });
//         //                     }
//         //                     None => continue,
//         //                 };
//         //             }
//         //             SourcePoll::InterruptPending => {
//         //                 self.wakers.state_interrupt_pending.push_back(input_hash);
//         //             }
//         //         }
//         //         continue;
//         //     }

//         //     // now we have no items in state_interrupt_woken.

//         //     if !self.wakers.state_interrupt_pending.is_empty() {
//         //         break Ok(SourcePoll::InterruptPending);
//         //     }

//         //     // println!("advance_upper_bound: {:?}", self.main.advance_upper_bound);

//         //     // start a step saturation if we need to.
//         //     if self.wakers.step_item.is_none() {
//         //         if let Some((step_a, step_b)) = self.main.steps.get_last_two_steps() {
//         //             if step_b.step.is_unsaturated()
//         //                 && self.main.advance_upper_bound.test(&step_b.step.get_time())
//         //             {
//         //                 step_b.step.start_saturate_clone(step_a).unwrap();
//         //                 self.wakers.step_item = Some(StepItem {
//         //                     step_uuid: step_b.uuid,
//         //                     step_woken: StepWokenStatus::Uninitialized,
//         //                     input_state_status: Status::None,
//         //                 });
//         //             } else {
//         //                 break Ok(SourcePoll::StateProgress {
//         //                     state: (),
//         //                     next_event_at: self.main.get_next_scheduled_time(),
//         //                     interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//         //                 });
//         //             }
//         //         } else {
//         //             break Ok(SourcePoll::StateProgress {
//         //                 state: (),
//         //                 next_event_at: self.main.get_next_scheduled_time(),
//         //                 interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//         //             });
//         //         }
//         //     }

//         //     // step polling (+ input states initiated by step polls)
//         //     if let Some(step_item) = &mut self.wakers.step_item {
//         //         match step_item.input_state_status {
//         //             Status::Woken {
//         //                 input_hash,
//         //                 input_channel,
//         //             } => {
//         //                 let step_source_waker = self
//         //                     .outer_wakers
//         //                     .get_source_step_waker(input_hash, step_item.step_uuid);
//         //                 let mut source = self
//         //                     .main
//         //                     .input_sources
//         //                     .get_input_by_hash(input_hash)
//         //                     .unwrap();
//         //                 let step_wrapper = self
//         //                     .main
//         //                     .steps
//         //                     .get_step_wrapper_mut_by_uuid(step_item.step_uuid)
//         //                     .unwrap();
//         //                 let time = step_wrapper.step.get_time();

//         //                 let context = SourceContext {
//         //                     channel: input_channel,
//         //                     channel_waker: step_source_waker.clone(),
//         //                     interrupt_waker: step_source_waker,
//         //                 };

//         //                 match source.poll(time, context).unwrap() {
//         //                     SourcePoll::StateProgress {
//         //                         state: Poll::Ready(state),
//         //                         ..
//         //                     } => {
//         //                         match step_wrapper.step.provide_input_state(state) {
//         //                             Ok(()) => {}
//         //                             Err(_) => panic!(),
//         //                         }
//         //                         step_item.input_state_status = Status::None;
//         //                         step_item.step_woken = StepWokenStatus::Woken;
//         //                     }
//         //                     SourcePoll::StateProgress {
//         //                         state: Poll::Pending,
//         //                         ..
//         //                     } => {
//         //                         break Ok(SourcePoll::InterruptPending);
//         //                     }
//         //                     SourcePoll::Interrupt {
//         //                         time, interrupt, ..
//         //                     } => {
//         //                         match self.handle_interrupt(input_hash, time, interrupt) {
//         //                             Some(mapped_time) => {
//         //                                 break Ok(SourcePoll::Interrupt {
//         //                                     time: mapped_time,
//         //                                     interrupt: Interrupt::Rollback,
//         //                                     interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//         //                                 });
//         //                             }
//         //                             None => continue,
//         //                         };
//         //                     }
//         //                     SourcePoll::InterruptPending => break Ok(SourcePoll::InterruptPending),
//         //                 }
//         //             }
//         //             Status::Pending { .. } => {
//         //                 break Ok(SourcePoll::InterruptPending);
//         //             }
//         //             Status::None => {}
//         //         }

//         //         if step_item.step_woken == StepWokenStatus::Pending {
//         //             break Ok(SourcePoll::InterruptPending);
//         //         }

//         //         let waker = self.outer_wakers.get_step_waker(step_item.step_uuid);

//         //         // // init step is handled slightly differently.
//         //         // if step_item.step_uuid == 0 {
//         //         //     let init_step = self.main.steps.get_init_step_mut();
//         //         //     let poll = init_step.poll(&waker).unwrap();
//         //         //     match poll {
//         //         //         StepPoll::Ready => {
//         //         //             if let Some(next_step) = init_step
//         //         //                 .next_unsaturated(&mut self.main.input_buffer)
//         //         //                 .unwrap()
//         //         //             {
//         //         //                 self.main.steps.push_step(next_step);
//         //         //             }
//         //         //             self.wakers.step_item = None;
//         //         //             continue;
//         //         //         }
//         //         //         StepPoll::Pending => {
//         //         //             break Ok(SourcePoll::InterruptPending);
//         //         //         }
//         //         //         _ => { panic!() }
//         //         //     }
//         //         // }

//         //         let step_wrapper = self
//         //             .main
//         //             .steps
//         //             .get_step_wrapper_mut_by_uuid(step_item.step_uuid)
//         //             .unwrap();

//         //         let poll = step_wrapper.step.poll(&waker).unwrap();
//         //         let step_time = step_wrapper.step.get_time();
//         //         match poll {
//         //             StepPoll::Ready => {
//         //                 if let Some(next_step) = step_wrapper
//         //                     .step
//         //                     .next_unsaturated(&mut self.main.input_buffer)
//         //                     .unwrap()
//         //                 {
//         //                     self.main.steps.push_step(next_step);
//         //                 }
//         //                 self.wakers.step_item = None;
//         //                 continue;
//         //             }
//         //             StepPoll::Emitted(e) => {
//         //                 break Ok(SourcePoll::Interrupt {
//         //                     time: step_time,
//         //                     interrupt: Interrupt::Event(e),
//         //                     interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//         //                 });
//         //             }
//         //             StepPoll::Pending => {
//         //                 break Ok(SourcePoll::InterruptPending);
//         //             }
//         //             StepPoll::StateRequested(input) => {
//         //                 let input_hash = input.get_hash();
//         //                 step_item.input_state_status = Status::Woken {
//         //                     input_hash,
//         //                     input_channel: 0,
//         //                 };
//         //                 continue;
//         //             }
//         //         }
//         //     }
//         // }
//     }

//     fn poll_inner(
//         &mut self,
//         // None for poll_interrupts
//         poll_time: T::Time,
//         cx: SourceContext,
//         forget: bool,
//     ) -> TrySourcePoll<T::Time, T::OutputEvent, Poll<T::OutputState>> {
//         if !self.main.advance_lower_bound.test(&poll_time) {
//             return Err(SourcePollErr::PollAfterAdvance);
//         }

//         loop {
//             match self.poll_interrupts_inner(&cx.interrupt_waker)? {
//                 SourcePoll::StateProgress { .. } => {}
//                 SourcePoll::Interrupt {
//                     time,
//                     interrupt,
//                     interrupt_lower_bound,
//                 } => {
//                     return Ok(SourcePoll::Interrupt {
//                         time,
//                         interrupt,
//                         interrupt_lower_bound,
//                     });
//                 }
//                 SourcePoll::InterruptPending => return Ok(SourcePoll::InterruptPending),
//             };

//             let channel_item = match self.wakers.channels.entry(cx.channel) {
//                 std::collections::hash_map::Entry::Occupied(occupied_entry) => {
//                     occupied_entry.into_mut()
//                 }
//                 std::collections::hash_map::Entry::Vacant(vacant_entry) => {
//                     let interpolation_uuid = self.main.next_interpolation_uuid;
//                     self.main.next_interpolation_uuid += 1;
//                     self.main.interpolations.insert(
//                         interpolation_uuid,
//                         (
//                             forget,
//                             Box::pin(self.main.steps.create_interpolation(poll_time)),
//                         ),
//                     );
//                     vacant_entry.insert(ChannelItem {
//                         interpolation_uuid,
//                         waker: cx.channel_waker.clone(),
//                         interpolation_woken: true,
//                         input_state_status: Status::None,
//                     })
//                 }
//             };

//             // step polling (+ input states initiated by step polls)
//             if let Status::Woken {
//                 input_hash,
//                 input_channel,
//             } = channel_item.input_state_status
//             {
//                 let source_channel_waker = self
//                     .outer_wakers
//                     .get_source_channel_waker(input_hash, channel_item.interpolation_uuid);
//                 let mut source = self
//                     .main
//                     .input_sources
//                     .get_input_by_hash(input_hash)
//                     .unwrap();
//                 let source_interrupt_waker =
//                     self.outer_wakers.get_source_interrupt_waker(input_hash);
//                 let (_, interpolation) = self
//                     .main
//                     .interpolations
//                     .get_mut(&channel_item.interpolation_uuid)
//                     .unwrap();

//                 debug_assert_eq!(poll_time, interpolation.get_time());

//                 let context = SourceContext {
//                     channel: input_channel,
//                     channel_waker: source_channel_waker,
//                     interrupt_waker: source_interrupt_waker,
//                 };

//                 let poll = if forget {
//                     source.poll_forget(poll_time, context)
//                 } else {
//                     source.poll(poll_time, context)
//                 };

//                 match poll.unwrap() {
//                     SourcePoll::StateProgress {
//                         state: Poll::Ready(state),
//                         ..
//                     } => {
//                         match interpolation
//                             .as_mut()
//                             .get_input_state_manager()
//                             .provide_input_state(state)
//                         {
//                             Ok(()) => {}
//                             Err(_) => panic!(),
//                         }
//                         self.main
//                             .channel_reservations
//                             .clear_channel(input_hash, input_channel);
//                         channel_item.input_state_status = Status::None;
//                         channel_item.interpolation_woken = true;
//                     }
//                     SourcePoll::StateProgress {
//                         state: Poll::Pending,
//                         ..
//                     } => {
//                         break Ok(SourcePoll::StateProgress {
//                             state: Poll::Pending,
//                             next_event_at: self.main.get_next_scheduled_time(),
//                             interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//                         });
//                     }
//                     SourcePoll::Interrupt {
//                         time, interrupt, ..
//                     } => {
//                         match self.handle_interrupt(input_hash, time, interrupt) {
//                             Some(interrupt_time) => {
//                                 break Ok(SourcePoll::Interrupt {
//                                     time: interrupt_time,
//                                     interrupt: Interrupt::Rollback,
//                                     interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//                                 });
//                             }
//                             None => continue,
//                         };
//                     }
//                     SourcePoll::InterruptPending => break Ok(SourcePoll::InterruptPending),
//                 }
//             }

//             if !channel_item.interpolation_woken {
//                 break Ok(SourcePoll::StateProgress {
//                     state: Poll::Pending,
//                     next_event_at: self.main.get_next_scheduled_time(),
//                     interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//                 });
//             }

//             let mut interpolation = self
//                 .main
//                 .interpolations
//                 .get_mut(&channel_item.interpolation_uuid)
//                 .unwrap()
//                 .1
//                 .as_mut();
//             let waker = self
//                 .outer_wakers
//                 .get_interpolation_waker(channel_item.interpolation_uuid);
//             match interpolation
//                 .as_mut()
//                 .poll(&mut Context::from_waker(&waker))
//             {
//                 Poll::Ready(state) => {
//                     self.main
//                         .interpolations
//                         .remove(&channel_item.interpolation_uuid);
//                     self.wakers.channels.remove(&cx.channel);
//                     self.main.returned_state_times.insert(poll_time);
//                     break Ok(SourcePoll::StateProgress {
//                         state: Poll::Ready(state),
//                         next_event_at: self.main.get_next_scheduled_time(),
//                         interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//                     });
//                 }
//                 Poll::Pending => {
//                     match interpolation.get_input_state_manager().try_accept_request() {
//                         Some(input) => {
//                             let input_hash = input.get_hash();
//                             let entry = self
//                                 .main
//                                 .channel_reservations
//                                 .get_first_available_channel(input_hash);
//                             let input_channel = entry.get().input_channel;
//                             entry.insert();
//                             channel_item.input_state_status = Status::Woken {
//                                 input_hash,
//                                 input_channel,
//                             };
//                         }
//                         None => {
//                             break Ok(SourcePoll::StateProgress {
//                                 state: Poll::Pending,
//                                 next_event_at: self.main.get_next_scheduled_time(),
//                                 interrupt_lower_bound: self.main.get_interrupt_lower_bound(),
//                             });
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

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
            match locked.main.input_sources.poll_aggregate_interrupts(|source_hash| {
                locked.outer_wakers.get_source_interrupt_waker(source_hash)
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
            match locked.main.working_timeline_slice.poll(|step_uuid| {
                locked.outer_wakers.get_step_waker(step_uuid)
            }) {
                WorkingTimelineSlicePoll::Emitted { time, event } => {
                    let interrupt_lower_bound = inputs_interrupt_lower_bound.min(locked.main.working_timeline_slice.tentative_state_and_event_lower_bound());
                    return TrySourcePoll::Ok(SourcePoll::Interrupt { time, interrupt: Interrupt::Event(event), interrupt_lower_bound })
                },
                WorkingTimelineSlicePoll::StateRequested { time, input, step_uuid } => todo!(),
                WorkingTimelineSlicePoll::Ready { next_time } => {
                    break 'steps next_time;
                },
                WorkingTimelineSlicePoll::Pending => return TrySourcePoll::Ok(SourcePoll::InterruptPending),
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
            locked.outer_wakers.get_source_interrupt_waker(source_hash)
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
