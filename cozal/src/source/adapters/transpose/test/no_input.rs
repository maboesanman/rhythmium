use std::{num::NonZeroUsize, task::Waker, time::Duration};

use crate::{
    source::{adapters::transpose::TransposeBuilder, traits::SourceContext, Source, SourcePoll},
    transposer::Transposer,
};

#[derive(Clone, Debug)]
struct CollatzTransposer {
    current_value: u64,
    times_incremented: u64,
    count_until_1: Option<u64>,
}

impl Transposer for CollatzTransposer {
    type Time = Duration;

    type OutputEvent = ();

    type OutputState = Option<u64>;

    type Scheduled = ();

    fn prepare_to_init(&mut self) -> bool {
        true
    }

    async fn init(&mut self, cx: &mut crate::transposer::InitContext<'_, Self>) {
        cx.schedule_event(Duration::from_secs(1), ()).unwrap();
    }

    async fn handle_scheduled_event(
        &mut self,
        _: Self::Scheduled,
        cx: &mut crate::transposer::HandleScheduleContext<'_, Self>,
    ) {
        cx.schedule_event(cx.current_time() + Duration::from_secs(1), ())
            .unwrap();
        // println!("transposer: {:?}", self);
        if self.current_value == 1 && self.count_until_1.is_none() {
            self.count_until_1 = Some(self.times_incremented);
        }
        self.times_incremented += 1;
        if self.current_value % 2 == 0 {
            self.current_value /= 2;
        } else {
            self.current_value = self.current_value * 3 + 1;
        }
    }

    async fn interpolate(
        &self,
        _cx: &mut crate::transposer::InterpolateContext<'_, Self>,
    ) -> Self::OutputState {
        self.count_until_1
    }
}

#[test]
fn transpose_no_inputs_no_events() {
    let mut transpose = TransposeBuilder::new(
        CollatzTransposer {
            current_value: 70,
            times_incremented: 0,
            count_until_1: None,
        },
        Duration::ZERO,
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .build()
    .unwrap();

    let context = SourceContext {
        channel: 0,
        channel_waker: Waker::noop().clone(),
        interrupt_waker: Waker::noop().clone(),
    };
    let poll = transpose.poll(Duration::from_secs_f32(70.0), context);

    assert!(matches!(
        poll,
        Ok(SourcePoll::Ready {
            state: Some(14),
            next_event_at: Some(_)
        })
    ));
}

#[derive(Clone, Debug)]
struct CollatzTransposer2 {
    current_value: u64,
    times_incremented: u64,
    count_until_1: Option<u64>,
}

impl Transposer for CollatzTransposer2 {
    type Time = Duration;

    type OutputEvent = u64;

    type OutputState = Option<u64>;

    type Scheduled = ();

    fn prepare_to_init(&mut self) -> bool {
        true
    }

    async fn init(&mut self, cx: &mut crate::transposer::InitContext<'_, Self>) {
        cx.schedule_event(Duration::from_secs(1), ()).unwrap();
    }

    async fn handle_scheduled_event(
        &mut self,
        _: Self::Scheduled,
        cx: &mut crate::transposer::HandleScheduleContext<'_, Self>,
    ) {
        cx.schedule_event(cx.current_time() + Duration::from_secs(1), ())
            .unwrap();
        // println!("transposer: {:?}", self);
        if self.current_value == 1 && self.count_until_1.is_none() {
            self.count_until_1 = Some(self.times_incremented);
        }
        self.times_incremented += 1;
        if self.current_value % 2 == 0 {
            self.current_value /= 2;
        } else {
            self.current_value = self.current_value * 3 + 1;
        }

        cx.emit_event(self.current_value).await;
    }

    async fn interpolate(
        &self,
        _cx: &mut crate::transposer::InterpolateContext<'_, Self>,
    ) -> Self::OutputState {
        self.count_until_1
    }
}

#[test]
fn transpose_no_inputs_with_events() {
    let mut transpose = TransposeBuilder::new(
        CollatzTransposer2 {
            current_value: 70,
            times_incremented: 0,
            count_until_1: None,
        },
        Duration::ZERO,
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .build()
    .unwrap();

    let context = SourceContext {
        channel: 0,
        channel_waker: Waker::noop().clone(),
        interrupt_waker: Waker::noop().clone(),
    };

    for _ in 0..100 {
        let poll = transpose
            .poll(Duration::from_secs_f32(70.0), context.clone())
            .unwrap();
        println!("{:?}", poll);
    }

    let poll = transpose
        .poll(Duration::from_secs_f32(14.5), context.clone())
        .unwrap();
    println!("{:?}", poll);
    let poll = transpose
        .poll(Duration::from_secs_f32(14.5), context.clone())
        .unwrap();
    println!("{:?}", poll);

    transpose.advance(Duration::from_secs_f32(30.0));
    let poll = transpose
        .poll(Duration::from_secs(35), context.clone())
        .unwrap();
    println!("{:?}", poll);

    transpose.advance(Duration::from_secs_f32(30.0));
    println!("{:?}", transpose.main.advance_time);
    let poll = transpose
        .poll(Duration::from_secs(25), context.clone())
        .unwrap_err();
    println!("{:?}", poll);
}

