use core::pin::Pin;

use super::super::{pre_init_step::PreInitStep, StepPoll};
use crate::util::dummy_waker::DummyWaker;
use archery::ArcTK;
use rand::Rng;

use crate::transposer::context::{HandleScheduleContext, InitContext, InterpolateContext};
use crate::transposer::step::Step;
use crate::transposer::Transposer;

#[derive(Clone, Debug)]
struct TestTransposer {
    counter: u32,
}

impl Transposer for TestTransposer {
    type Time = u32;

    type OutputState = u32;

    type Scheduled = ();

    type OutputEvent = ();

    fn prepare_to_init(&mut self) -> bool {
        true
    }

    async fn init(&mut self, cx: &mut InitContext<'_, Self>) {
        self.counter = 0;
        cx.schedule_event(1, ()).unwrap();
    }

    async fn handle_scheduled_event(
        &mut self,
        _payload: Self::Scheduled,
        cx: &mut HandleScheduleContext<'_, Self>,
    ) {
        cx.schedule_event(cx.current_time() + 1, ()).unwrap();

        self.counter += 1;
        cx.emit_event(()).await;
    }

    async fn interpolate(&self, _cx: &mut InterpolateContext<'_, Self>) -> Self::OutputState {
        self.counter
    }
}

#[test]
fn next_scheduled_unsaturated_take() {
    let transposer = TestTransposer { counter: 17 };
    let rng_seed = rand::thread_rng().gen();

    let mut step = Step::<_, ArcTK>::new_init(transposer, PreInitStep::new(), 0, rng_seed).unwrap();

    let waker = DummyWaker::dummy();
    Pin::new(&mut step).poll(&waker).unwrap();

    for i in 1..100 {
        let mut next = step.next_scheduled_unsaturated().unwrap().unwrap();
        next.start_saturate_take(&mut step).unwrap();

        assert!(matches!(next.poll(&waker), Ok(StepPoll::Emitted(()))));
        assert!(matches!(next.poll(&waker), Ok(StepPoll::Ready)));

        let interpolated = futures_executor::block_on(next.interpolate(i + 1).unwrap());
        assert_eq!(interpolated, i);

        step = next;
    }
}

#[test]
fn next_scheduled_unsaturated_clone() {
    let transposer = TestTransposer { counter: 17 };
    let rng_seed = rand::thread_rng().gen();

    let mut step = Step::<_, ArcTK>::new_init(transposer, PreInitStep::new(), 0, rng_seed).unwrap();

    let waker = DummyWaker::dummy();
    Pin::new(&mut step).poll(&waker).unwrap();

    for i in 1..100 {
        let mut next = step.next_scheduled_unsaturated().unwrap().unwrap();
        next.start_saturate_clone(&step).unwrap();

        assert!(matches!(next.poll(&waker), Ok(StepPoll::Emitted(()))));
        assert!(matches!(next.poll(&waker), Ok(StepPoll::Ready)));

        let interpolated = futures_executor::block_on(next.interpolate(i + 1).unwrap());
        assert_eq!(interpolated, i);

        step = next;
    }
}

#[test]
fn next_scheduled_unsaturated_desaturate() {
    let transposer = TestTransposer { counter: 17 };
    let rng_seed = rand::thread_rng().gen();

    let mut init = Step::<_, ArcTK>::new_init(transposer, PreInitStep::new(), 0, rng_seed).unwrap();

    let waker = DummyWaker::dummy();
    Pin::new(&mut init).poll(&waker).unwrap();

    let mut step1 = init.next_scheduled_unsaturated().unwrap().unwrap();
    step1.start_saturate_clone(&init).unwrap();

    // emits the event the first time
    assert!(matches!(step1.poll(&waker), Ok(StepPoll::Emitted(()))));
    assert!(matches!(step1.poll(&waker), Ok(StepPoll::Ready)));

    step1.desaturate();
    step1.start_saturate_clone(&init).unwrap();

    // doesn't re-emit the event
    assert!(matches!(step1.poll(&waker), Ok(StepPoll::Ready)));

    step1.desaturate();
    step1.start_saturate_clone(&init).unwrap();

    // doesn't re-emit the event
    assert!(matches!(step1.poll(&waker), Ok(StepPoll::Ready)));

    step1.desaturate();
}
