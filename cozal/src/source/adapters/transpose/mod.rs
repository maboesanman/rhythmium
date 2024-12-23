#![allow(dead_code)]

use std::collections::{BTreeSet, HashSet};

use archery::ArcTK;
use erased_input_source::{ErasedInputSource, ErasedInputSourceCollection};
use steps::Steps;

// // mod caller_channel_status;
// // mod channel_assignments;
// // mod channels;
// // mod input_buffer;
// // mod retention_policy;
mod steps;
// // mod storage;
// // mod transpose_step_metadata;
mod erased_input_source;

use std::any::Any;
use std::pin::Pin;
use std::sync::Weak;
use std::task::{Poll, Waker};

use crate::source::source_poll::TrySourcePoll;
use crate::source::traits::SourceContext;
use crate::source::{Source, SourcePoll};
use crate::transposer::step::BoxedInput;
// use pin_project::pin_project;
use crate::transposer::Transposer;
use crate::util::replace_waker::ReplaceWaker;
use crate::util::stack_waker::StackWaker;

// use self::channels::ChannelStatuses;
// use self::input_buffer::InputBuffer;
// use self::retention_policy::RetentionPolicy;
// use self::steps::Steps;
// use crate::adapters::transpose::channels::CallerChannelStatus;
// use crate::source_poll::{Interrupt, SourcePoll, TrySourcePoll};
// use crate::traits::SourceContext;
// use crate::Source;

// struct InputSource {
//     input_type: TypeId,
//     input_hash: u64,

// }

pub struct Transpose<T: Transposer + 'static> {

    // The sources we are transposing.
    input_sources: ErasedInputSourceCollection<T>,

    // the working steps
    steps: Steps<T>,

    // the inputs not yet consumed by the steps
    input_buffer: BTreeSet<BoxedInput<'static, T, ArcTK>>,


    last_scheduled: Option<T::Time>,

    // the all channel waker to keep up to date.
    all_channel_waker: Weak<ReplaceWaker>,

    // the time to use for poll_events calls.
    // this should be the time of the latest emitted state,
    // or the currently saturating/unsaturated "original" step,
    // whichever is later
    events_poll_time: T::Time,

    // current channel obligations
    // channel_statuses: ChannelStatuses<T>,
}

enum TrySourcePollToHandle {}

impl<T> Transpose<T>
where
    T: Transposer + Clone + 'static,
{
    fn new(inputs: HashSet<ErasedInputSource<T>>, transposer: T, rng_seed: [u8; 32]) -> Self {
        // Self {
        //     source,
        //     last_scheduled: None,
        //     all_channel_waker: ReplaceWaker::new_empty(),
        //     events_poll_time: T::Time::default(),
        //     channel_statuses: ChannelStatuses::new(),
        //     steps: Steps::new(transposer, rng_seed),
        //     input_buffer: InputBuffer::new(),
        // }

        todo!()
    }

    fn ready_or_scheduled(
        &self,
        state: T::OutputState,
    ) -> SourcePoll<T::Time, T::OutputEvent, T::OutputState> {
        // match (
        //     self.channel_statuses.get_scheduled_time(),
        //     self.last_scheduled,
        // ) {
        //     (None, None) => SourcePollOk::Ready(state),
        //     (None, Some(t)) => SourcePollOk::Scheduled(state, t),
        //     (Some(t), None) => SourcePollOk::Scheduled(state, t),
        //     (Some(t1), Some(t2)) => SourcePollOk::Scheduled(state, std::cmp::min(t1, t2)),
        // }
        todo!()
    }

    fn poll_inner(
        &mut self,
        poll_time: T::Time,
        cx: SourceContext,
        forget: bool,
    ) -> TrySourcePoll<T::Time, T::OutputEvent, T::OutputState> {
        todo!()
        // let Transpose {
        //     mut source,
        //     last_scheduled,
        //     all_channel_waker,
        //     events_poll_time,
        //     channel_statuses,
        //     steps,
        //     input_buffer,
        // } = self.as_mut().project();

        // let SourceContext {
        //     channel: caller_channel,
        //     one_channel_waker,
        //     all_channel_waker: caller_all_channel_waker,
        // } = cx;

        // let mut unhandled_interrupt: Option<(Src::Time, Interrupt<Src::Event>)> = None;

        // // poll events if our all channel waker was triggered.
        // if let Some(waker) = ReplaceWaker::register(all_channel_waker, caller_all_channel_waker) {
        //     match source.as_mut().poll_events(*events_poll_time, waker)? {
        //         SourcePoll::Ready {
        //             state: (),
        //             next_event_at,
        //         } => *last_scheduled = next_event_at,
        //         SourcePoll::Interrupt {
        //             time,
        //             interrupt,
        //         } => {
        //             debug_assert!(unhandled_interrupt.is_none());
        //             unhandled_interrupt = Some((time, interrupt))
        //         },
        //         SourcePoll::Pending => return Ok(SourcePoll::Pending),
        //     }
        // }

        // // at this point we only need to poll the source if state is needed.
        // // we are ready to start manipulating the status,
        // // handling blockers as they arise.

        // let mut status = channel_statuses.get_channel_status(caller_channel);

        // let all_channel_waker = ReplaceWaker::get_waker(all_channel_waker);

        // loop {
        //     if let Some((time, interrupt)) = unhandled_interrupt.take() {
        //         match interrupt {
        //             Interrupt::Event(e) => input_buffer.insert_back(time, e),
        //             Interrupt::Rollback => input_buffer.rollback(time),
        //             Interrupt::Finalize => {
        //                 // let should_release_old_steps = retention_policy.source_finalize(time);
        //                 // if should_release_old_steps {
        //                 //     todo!()
        //                 // }
        //                 todo!();
        //                 return Ok(SourcePoll::Interrupt {
        //                     time,
        //                     interrupt: Interrupt::Finalize,
        //                 })
        //             },
        //         }
        //     }

        //     match core::mem::replace(&mut status, CallerChannelStatus::Limbo) {
        //         CallerChannelStatus::Limbo => unreachable!(),
        //         CallerChannelStatus::Free(inner_status) => {
        //             status = inner_status.poll(forget, poll_time);
        //         },
        //         CallerChannelStatus::OriginalStepSourceState(mut inner_status) => {
        //             let (time, source_channel) = inner_status.get_args_for_source_poll();

        //             // original steps can emit events which effect all channels,
        //             // so this uses the all channel waker for both of these.
        //             let cx = SourceContext {
        //                 channel:           source_channel,
        //                 one_channel_waker: all_channel_waker.clone(),
        //                 all_channel_waker: all_channel_waker.clone(),
        //             };

        //             let state = match source.as_mut().poll(time, cx)? {
        //                 SourcePoll::Ready {
        //                     state,
        //                     next_event_at,
        //                 } => {
        //                     *last_scheduled = next_event_at;
        //                     state
        //                 },
        //                 SourcePoll::Interrupt {
        //                     time,
        //                     interrupt,
        //                 } => {
        //                     unhandled_interrupt = Some((time, interrupt));
        //                     continue
        //                 },
        //                 SourcePoll::Pending => return Ok(SourcePoll::Pending),
        //             };

        //             // this provide state call will not poll the future.
        //             let inner_status = inner_status.provide_state(state);

        //             // now loop again, polling the future on the next pass.
        //             status = CallerChannelStatus::OriginalStepFuture(inner_status);
        //         },
        //         CallerChannelStatus::OriginalStepFuture(inner_status) => {
        //             todo!()
        //             // let t = inner_status.time();

        //             // // get the first item, so it can be pulled if needed by poll
        //             // // (if original completes it needs to make a new original future)
        //             // let mut first = input_buffer.pop_first();

        //             // let (s, output) = inner_status.poll(&all_channel_waker, &mut first);

        //             // // if poll didn't need the input, put it back in the buffer
        //             // if let Some((t, inputs)) = first {
        //             //     input_buffer.extend_front(t, inputs)
        //             // }

        //             // // handle all the generated outputs
        //             // if let Some(output) = output {
        //             //     return Poll::Ready(Ok(SourcePollOk::Event(output, t)))
        //             // }

        //             // status = s;
        //         },
        //         CallerChannelStatus::RepeatStepSourceState(mut inner_status) => {
        //             let (time, stack_waker, source_channel) =
        //                 inner_status.get_args_for_source_poll();

        //             let stacked_waker = match StackWaker::register(
        //                 stack_waker,
        //                 caller_channel,
        //                 one_channel_waker.clone(),
        //             ) {
        //                 Some(w) => w,
        //                 None => return Ok(SourcePoll::Pending),
        //             };

        //             let cx = SourceContext {
        //                 channel:           source_channel,
        //                 one_channel_waker: stacked_waker,
        //                 all_channel_waker: all_channel_waker.clone(),
        //             };

        //             let state = match source.as_mut().poll(time, cx)? {
        //                 SourcePoll::Ready {
        //                     state,
        //                     next_event_at,
        //                 } => {
        //                     *last_scheduled = next_event_at;
        //                     state
        //                 },
        //                 SourcePoll::Interrupt {
        //                     time,
        //                     interrupt,
        //                 } => {
        //                     unhandled_interrupt = Some((time, interrupt));
        //                     continue
        //                 },
        //                 SourcePoll::Pending => return Ok(SourcePoll::Pending),
        //             };

        //             // this provide state call will not poll the future.
        //             let inner_status = inner_status.provide_state(state);

        //             // now loop again, polling the future on the next pass.
        //             status = CallerChannelStatus::RepeatStepFuture(inner_status);
        //         },
        //         CallerChannelStatus::RepeatStepFuture(inner_status) => {
        //             status = match inner_status.poll(&one_channel_waker) {
        //                 Poll::Ready(status) => status,
        //                 Poll::Pending => return Ok(SourcePoll::Pending),
        //             };
        //         },
        //         CallerChannelStatus::InterpolationSourceState(mut inner_status) => {
        //             let (source_channel, never_remebered) =
        //                 inner_status.get_args_for_source_poll(forget);

        //             let cx = SourceContext {
        //                 channel:           source_channel,
        //                 one_channel_waker: one_channel_waker.clone(),
        //                 all_channel_waker: all_channel_waker.clone(),
        //             };

        //             let poll = if never_remebered {
        //                 source.as_mut().poll_forget(poll_time, cx)
        //             } else {
        //                 source.as_mut().poll(poll_time, cx)
        //             }?;

        //             let state = match poll {
        //                 SourcePoll::Ready {
        //                     state,
        //                     next_event_at,
        //                 } => {
        //                     *last_scheduled = next_event_at;
        //                     state
        //                 },
        //                 SourcePoll::Interrupt {
        //                     time,
        //                     interrupt,
        //                 } => {
        //                     unhandled_interrupt = Some((time, interrupt));
        //                     continue
        //                 },
        //                 SourcePoll::Pending => return Ok(SourcePoll::Pending),
        //             };

        //             let inner_status = inner_status.provide_state(state);

        //             // now loop again, polling the future on the next pass.
        //             status = CallerChannelStatus::InterpolationFuture(inner_status);
        //         },
        //         CallerChannelStatus::InterpolationFuture(inner_status) => {
        //             let output_state = match inner_status.poll(&one_channel_waker) {
        //                 Ok(Poll::Ready(output_state)) => output_state,
        //                 Ok(Poll::Pending) => return Ok(SourcePoll::Pending),
        //                 Err(s) => {
        //                     status = s;
        //                     continue
        //                 },
        //             };

        //             return Ok(self.ready_or_scheduled(output_state))
        //         },
        //     }
        // }
    }
}

impl<T> Source for Transpose<T>
where
    T: Transposer + Clone + 'static,
{
    type Time = T::Time;

    type Event = T::OutputEvent;

    type State = T::OutputState;

    fn poll(
        &mut self,
        poll_time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        self.poll_inner(poll_time, cx, false)
    }

    fn poll_forget(
        &mut self,
        poll_time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        self.poll_inner(poll_time, cx, true)
    }

    fn poll_events(
        &mut self,
        poll_time: Self::Time,
        all_channel_waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        todo!(/* some variant of poll_inner? */)
    }

    fn advance(&mut self, time: Self::Time) {
        todo!(/*
            move the caller advance header, mark old steps for deletion
        */)
    }

    fn max_channel(&mut self) -> std::num::NonZeroUsize {
        todo!()
    }

    fn release_channel(&mut self, channel: usize) {
        todo!(/*
            delete the entry from the channel statuses,
            and forward the held channel if it was exclusively held
        */)
    }
}
