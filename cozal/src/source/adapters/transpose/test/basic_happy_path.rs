use std::{num::NonZeroUsize, time::Duration};

use futures::StreamExt;
use futures_test::future::FutureTestExt;

use crate::{
    source::{
        Source,
        adapters::{event_stream::into_event_stream, transpose::TransposeBuilder},
        source_poll::{LowerBound, UpperBound},
    },
    transposer::{Transposer, TransposerInput, TransposerInputEventHandler},
};

#[derive(Clone, Debug)]
struct CollatzTransposer {
    current_value: u64,
    times_incremented: u64,
    count_until_1: Option<u64>,
}

impl Transposer for CollatzTransposer {
    type Time = Duration;

    type OutputEvent = u64;

    type OutputState = ();

    type Scheduled = ();

    fn prepare_to_init(&mut self) -> bool {
        true
    }

    async fn init(&mut self, cx: &mut crate::transposer::InitContext<'_, Self>) {
        async move {
            cx.schedule_event(Duration::from_secs(0), ());
        }
        .pending_once()
        .await
    }

    async fn handle_scheduled_event(
        &mut self,
        _: Self::Scheduled,
        cx: &mut crate::transposer::HandleScheduleContext<'_, Self>,
    ) {
        async move {
            cx.emit_event(self.current_value).await;
            if self.current_value != 1 {
                cx.schedule_event(cx.current_time() + Duration::from_secs(1), ())
                    .unwrap();
            } else {
                self.count_until_1 = Some(self.times_incremented);
            }
            self.times_incremented += 1;
            if self.current_value % 2 == 0 {
                self.current_value /= 2;
            } else {
                self.current_value = self.current_value * 3 + 1;
            }
        }
        .pending_once()
        .await
    }

    async fn interpolate(
        &self,
        _cx: &mut crate::transposer::InterpolateContext<'_, Self>,
    ) -> Self::OutputState {
    }
}

#[tokio::test]
async fn test_stream() {
    let transpose = TransposeBuilder::new(
        CollatzTransposer {
            current_value: 70,
            times_incremented: 0,
            count_until_1: None,
        },
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .build()
    .unwrap();

    let mut stream = into_event_stream(transpose);

    while let Some(item) = stream.next().await {
        println!("item: {:?}", item);
    }

    println!("complete")
}

#[derive(Clone, Debug, Default)]
struct CollatzTransposerDelayer {
    input_registered: bool,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Debug)]
struct CollatzTransposerDelayerInput;

impl Transposer for CollatzTransposerDelayer {
    type Time = Duration;

    type OutputEvent = u64;

    type OutputState = ();

    type Scheduled = u64;

    fn prepare_to_init(&mut self) -> bool {
        true
    }

    async fn init(&mut self, _cx: &mut crate::transposer::InitContext<'_, Self>) {
        async move {}.pending_once().await
    }

    async fn handle_scheduled_event(
        &mut self,
        val: Self::Scheduled,
        cx: &mut crate::transposer::HandleScheduleContext<'_, Self>,
    ) {
        async move {
            cx.emit_event(val).await;
        }
        .pending_once()
        .await
    }

    async fn interpolate(
        &self,
        _cx: &mut crate::transposer::InterpolateContext<'_, Self>,
    ) -> Self::OutputState {
    }
}

impl TransposerInput for CollatzTransposerDelayerInput {
    type Base = CollatzTransposerDelayer;

    type InputEvent = u64;

    type InputState = ();

    const SORT: u64 = 0;
}

impl TransposerInputEventHandler<CollatzTransposerDelayerInput> for CollatzTransposerDelayer {
    fn register_input(&mut self, _input: CollatzTransposerDelayerInput) -> bool {
        let return_val = !self.input_registered;
        self.input_registered = true;
        return return_val;
    }

    async fn handle_input_event(
        &mut self,
        _input: &CollatzTransposerDelayerInput,
        event: &<CollatzTransposerDelayerInput as TransposerInput>::InputEvent,
        cx: &mut crate::transposer::HandleInputContext<'_, Self>,
    ) {
        async move {
            let scheduled_time = cx.current_time() + Duration::from_millis(350);
            cx.schedule_event(scheduled_time, *event).unwrap();
        }
        .pending_once()
        .await
    }
}

#[tokio::test]
async fn test_stream_composed() {
    let transpose = TransposeBuilder::new(
        CollatzTransposer {
            current_value: 70,
            times_incremented: 0,
            count_until_1: None,
        },
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .build()
    .unwrap();

    let transpose = TransposeBuilder::new(
        CollatzTransposerDelayer::default(),
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .add_input(CollatzTransposerDelayerInput, transpose)
    .ok()
    .unwrap()
    .build()
    .unwrap();

    let mut stream = into_event_stream(transpose);

    while let Some(item) = stream.next().await {
        println!("item: {:?}", item);
    }

    println!("complete")
}

#[test]
fn test_manual_composed() {
    let transpose = TransposeBuilder::new(
        CollatzTransposer {
            current_value: 70,
            times_incremented: 0,
            count_until_1: None,
        },
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .build()
    .unwrap();

    let mut transpose = TransposeBuilder::new(
        CollatzTransposerDelayer::default(),
        [69; 32],
        NonZeroUsize::new(1).unwrap(),
    )
    .add_input(CollatzTransposerDelayerInput, transpose)
    .ok()
    .unwrap()
    .build()
    .unwrap();

    transpose.advance_poll_lower_bound(LowerBound::max());

    for _ in 0..20 {
        let (count_waker, awoken_count) = futures_test::task::new_count_waker();
        let poll_result = transpose.poll_interrupts(count_waker).unwrap();

        match &poll_result {
            crate::source::SourcePoll::StateProgress {
                state,
                next_event_at,
                interrupt_lower_bound,
            } => {
                if let Some(at) = next_event_at {
                    let (count_waker, awoken_count) = futures_test::task::new_count_waker();
                    transpose
                        .advance_interrupt_upper_bound(UpperBound::inclusive(*at), count_waker);
                    println!("advance {:?}", awoken_count);
                }
            }
            crate::source::SourcePoll::Interrupt {
                time,
                interrupt,
                interrupt_lower_bound,
            } => {}
            crate::source::SourcePoll::InterruptPending => {}
        }

        println!("{:?}", poll_result);
        println!("{:?}", awoken_count);
    }

    println!("complete")
}
