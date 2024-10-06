use core::pin::Pin;

use crate::util::dummy_waker::DummyWaker;
use archery::ArcTK;
use rand::Rng;
use super::super::{pre_init_step::PreInitStep, StepPoll};

use crate::transposer::context::{HandleInputContext, HandleScheduleContext, InitContext, InterpolateContext};
use crate::transposer::step::Step;
use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

#[derive(Clone, Debug)]
struct TestTransposer {
    counter: u32,
}

impl Transposer for TestTransposer {
    type Time = u32;

    type OutputState = u32;

    type Scheduled = ();

    type OutputEvent = u8;
    
    fn prepare_to_init(&mut self) -> bool {
        true
    }

    async fn init(&mut self,  cx: &mut InitContext<'_, Self>) {
        self.counter = 0;
        cx.schedule_event(1, ()).unwrap();
    }

    async fn handle_scheduled_event(
        &mut self,
        _payload: Self::Scheduled,
         cx:&mut  HandleScheduleContext<'_, Self>,
    ) {
        for i in 0..10 {
            cx.emit_event(i).await;
        }
        cx.schedule_event(cx.current_time() + 1, ()).unwrap();

        self.counter += 1;
    }

    async fn interpolate(&self,  _cx: &mut InterpolateContext<'_, Self>) -> Self::OutputState {
        self.counter
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct TestTransposerInput;

impl TransposerInput for TestTransposerInput {
    type Base = TestTransposer;
    
    type InputEvent = u8;
    
    type InputState = u8;
    
    const SORT: u64 = 0;
}

impl TransposerInputEventHandler<TestTransposerInput> for TestTransposer {
    fn register_input(&mut self, input: TestTransposerInput) -> bool {
        todo!()
    }

    async fn handle_input_event(
        &mut self,
        input: &TestTransposerInput,
        event: &u8,
        cx: &mut HandleInputContext<'_, Self>,
    ) {
        todo!()
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
    for _i in 0..5 {
        assert!(matches!(step1.poll(&waker), Ok(StepPoll::Emitted(_i))));
    }
    step1.desaturate();
    step1.start_saturate_clone(&init).unwrap();
    for _i in 5..10 {
        assert!(matches!(step1.poll(&waker), Ok(StepPoll::Emitted(_i))));
    }
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
