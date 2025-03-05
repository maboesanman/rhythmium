use std::{task::Waker, time::Duration};

use crate::{
    source::{traits::SourceContext, Source},
    transposer::Transposer,
};

use super::TransposeBuilder;

#[derive(Clone)]
struct CollatzTransposer {
    current_value: u64,
}

impl Transposer for CollatzTransposer {
    type Time = Duration;

    type OutputEvent = (u64, bool);

    type OutputState = u64;

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
        self.current_value
    }
}

#[test]
fn transpose_no_inputs() {
    let mut transpose = TransposeBuilder::new(
        CollatzTransposer { current_value: 70 },
        Duration::ZERO,
        [69; 32],
    )
    .build()
    .unwrap();

    let context = SourceContext {
        channel: 0,
        channel_waker: Waker::noop().clone(),
        interrupt_waker: Waker::noop().clone(),
    };
    let poll = transpose.poll(Duration::from_secs_f32(5.5), context);

    println!("poll: {:?}", poll);
}
