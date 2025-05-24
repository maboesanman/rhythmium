use std::{
    assert_matches::assert_matches, io::Write, num::NonZeroUsize, sync::atomic::AtomicBool, task::{Context, Poll, Waker}, time::{Duration, Instant}
};

use futures::{FutureExt, StreamExt};
use futures_test::future::FutureTestExt;
use tokio::time::sleep_until;

use crate::{
    source::{
        adapters::{event_stream::{self, into_event_stream}, transpose::TransposeBuilder}, source_poll::{Interrupt, LowerBound, SourceBound, UpperBound}, traits::SourceContext, Source, SourcePoll
    },
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
        cx.schedule_event(Duration::from_secs(0), ());
    }

    async fn handle_scheduled_event(
        &mut self,
        _: Self::Scheduled,
        cx: &mut crate::transposer::HandleScheduleContext<'_, Self>,
    ) {
        cx.schedule_event(cx.current_time() + Duration::from_secs(1), ())
            .unwrap();
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

    println!("{poll:?}");

    assert!(matches!(
        poll,
        Ok(SourcePoll::StateProgress {
            state: Poll::Ready(Some(14)),
            next_event_at: Some(_),
            interrupt_lower_bound: LowerBound(SourceBound::Exclusive(_)),
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
        async move {
            cx.schedule_event(Duration::from_secs(0), ());
        }.pending_once().await
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
        async move {
            self.count_until_1
        }.pending_once()
        .await
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
            .poll(Duration::from_secs_f32(14.0), context.clone())
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

    transpose.advance_poll_lower_bound(
        LowerBound::exclusive(Duration::from_secs_f32(30.0)),
    );

    let poll = transpose
        .poll(Duration::from_secs(35), context.clone())
        .unwrap();
    println!("{:?}", poll);

    let poll = transpose
        .poll(Duration::from_secs(25), context.clone())
        .unwrap_err();
    println!("{:?}", poll);
}

#[tokio::test]
async fn test_stream() {
    let transpose = TransposeBuilder::new(
        CollatzTransposer2 {
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


// #[tokio::test]
// async fn test_stream_wakers() {
//     let transpose = TransposeBuilder::new(
//         CollatzTransposer2 {
//             current_value: 70,
//             times_incremented: 0,
//             count_until_1: None,
//         },
//         [69; 32],
//         NonZeroUsize::new(1).unwrap(),
//     )
//     .build()
//     .unwrap();

//     let start = Instant::now();

//     let mut stream = EventStream::new(transpose, start, |t| {
//         println!("sleep requested");
//         async move {
//             println!("sleep starting");
//             let x = sleep_until(t.into()).await;
//             println!("sleep over");
//             x
//         }.boxed()
//     });

//     let (waker, count) = futures_test::task::new_count_waker();
//     let mut context = Context::from_waker(&waker);

//     let poll = stream.poll_next_unpin(&mut context);

//     println!("{count:?}");
// }

// #[tokio::test]
// async fn basic_step_only_waker_test() {
//     let mut transpose = TransposeBuilder::new(
//         CollatzTransposer2 {
//             current_value: 70,
//             times_incremented: 0,
//             count_until_1: None,
//         },
//         [69; 32],
//         NonZeroUsize::new(1).unwrap(),
//     )
//     .build()
//     .unwrap();

//     // first call completes, and lets us know the next event is at time 0
//     let (waker1, count1) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker1);
//     assert_eq!(poll, 
//         Ok(SourcePoll::StateProgress {
//             state: (),
//             next_event_at: Some(Duration::ZERO),
//             interrupt_lower_bound: LowerBound::inclusive(Duration::ZERO)
//         }));
//     assert_eq!(count1.get(), 0);

//     // advancing into the new time invokes the waker, since there are new events.
//     let (waker2, count2) = futures_test::task::new_count_waker();
//     transpose.advance_interrupt_upper_bound(
//         UpperBound::inclusive(Duration::from_secs_f64(0.5)),
//         waker2,
//     );
//     assert_eq!(count2.get(), 1);

//     // first poll auto pendings, and immediately sends a wake
//     let (waker3, count3) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker3);
//     assert_eq!(poll, Ok(SourcePoll::InterruptPending));
//     assert_eq!(count3.get(), 1);

//     // second poll goes through
//     let (waker4, count4) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker4);
//     assert_eq!(poll, 
//         Ok(SourcePoll::Interrupt {
//             time: Duration::ZERO, interrupt: Interrupt::Event(70), 
//             interrupt_lower_bound: LowerBound::inclusive(Duration::ZERO) }
//     ));
//     assert_eq!(count4.get(), 0);

//     // third poll returns state progress
//     let (waker5, count5) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker5);
//     assert_eq!(poll, 
//         Ok(SourcePoll::StateProgress {
//             state: (),
//             next_event_at: Some(Duration::from_secs(1)),
//             interrupt_lower_bound: LowerBound::inclusive(Duration::from_secs(1))
//         }));
//     assert_eq!(count5.get(), 0);

//     // advancing into the new time invokes the waker, since there are new events.
//     let (waker6, count6) = futures_test::task::new_count_waker();
//     transpose.advance_interrupt_upper_bound(
//         UpperBound::inclusive(Duration::from_secs_f64(1.5)),
//         waker6,
//     );
//     assert_eq!(count6.get(), 1);

//     // first poll auto pendings, and immediately sends a wake
//     let (waker7, count7) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker7);
//     assert_eq!(poll, Ok(SourcePoll::InterruptPending));
//     assert_eq!(count7.get(), 1);

//     // second poll goes through
//     let (waker8, count8) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker8);
//     assert_eq!(poll, 
//         Ok(SourcePoll::Interrupt {
//             time: Duration::from_secs(1), interrupt: Interrupt::Event(35), 
//             interrupt_lower_bound: LowerBound::inclusive(Duration::from_secs(1)) }
//     ));
//     assert_eq!(count8.get(), 0);

//     // third poll returns state progress
//     let (waker9, count9) = futures_test::task::new_count_waker();
//     let poll = transpose.poll_interrupts(waker9);
//     assert_eq!(poll, 
//         Ok(SourcePoll::StateProgress {
//             state: (),
//             next_event_at: Some(Duration::from_secs(2)),
//             interrupt_lower_bound: LowerBound::inclusive(Duration::from_secs(2))
//         }));
//     assert_eq!(count9.get(), 0);
// }

