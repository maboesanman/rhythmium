use std::{num::NonZeroUsize, task::Waker, time::Duration};

use futures_test::future::FutureTestExt;

use crate::{source::{adapters::{state_function_source::StateFunctionSource, transpose::TransposeBuilder}, traits::SourceContext, Source, SourcePoll}, transposer::{Transposer, TransposerInput, TransposerInputEventHandler}};


#[derive(Clone, Debug)]
struct CollatzTransposer {
    current_value: u64,
    times_incremented: u64,
    count_until_1: Option<u64>,

    input_provided: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct CollatzInput;

impl TransposerInput for CollatzInput {
    type Base = CollatzTransposer;

    type InputEvent = ();

    type InputState = String;

    const SORT: u64 = 0;
}

impl Transposer for CollatzTransposer {
    type Time = Duration;

    type OutputEvent = u64;

    type OutputState = String;

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
        cx.emit_event(self.current_value).await;
        if self.current_value != 1 {
            cx.schedule_event(cx.current_time() + Duration::from_secs(1), ())
                .unwrap();
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
        cx: &mut crate::transposer::InterpolateContext<'_, Self>,
    ) -> Self::OutputState {
        async {
            let prefix = cx.get_input_state(CollatzInput).await;
            format!(
                "{}: {}",
                prefix,
                self.current_value
            )
        }.pending_once().await
    }
}

impl TransposerInputEventHandler<CollatzInput> for CollatzTransposer {
    fn register_input(&mut self, _input: CollatzInput) -> bool {
        if self.input_provided {
            return false;
        }

        self.input_provided = true;
        return true;
    }

    async fn handle_input_event(
        &mut self,
        _input: &CollatzInput,
        _event: &<CollatzInput as TransposerInput>::InputEvent,
        _cx: &mut crate::transposer::HandleInputContext<'_, Self>,
    ) {
        // it doesn't make any events anyway.
    }
}

#[test]
fn transpose_state_only_input() {
    let mut builder = TransposeBuilder::new(
        CollatzTransposer {
            current_value: 70,
            times_incremented: 0,
            count_until_1: None,
            input_provided: false,
        },
        Duration::ZERO,
        [69; 32],
        NonZeroUsize::new(2).unwrap(),
    );
    match builder.add_input(CollatzInput, StateFunctionSource::new(|d| {
        format!("Collatz({:?})", d)
    })) {
        Ok(_) => {},
        Err(_) => panic!(),
    };
    let mut transpose = builder.build().unwrap();

    let context = SourceContext {
        channel: 0,
        channel_waker: Waker::noop().clone(),
        interrupt_waker: Waker::noop().clone(),
    };

    for _ in 0..50 {
        let poll = transpose.poll(Duration::from_secs_f32(70.0), context.clone());
        println!("{:?}", poll);
    }
}