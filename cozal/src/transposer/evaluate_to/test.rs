use std::time::Duration;

use futures_test::future::FutureTestExt;

use crate::transposer::Transposer;


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
        cx.schedule_event(Duration::from_secs(0), ());
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
        self.count_until_1
    }
}