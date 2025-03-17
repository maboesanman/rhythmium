use std::ptr::NonNull;

use archery::SharedPointerKind;

use super::init_context::InitUpdateContext;
use super::sub_step_update_context::SubStepUpdateContext;
use super::time::SubStepTime;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::{HandleInputContext, HandleScheduleContext, InitContext};
use crate::transposer::input_state_manager::InputStateManager;
use crate::transposer::output_event_manager::OutputEventManager;
use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

// #[derive(Clone)]
pub struct WrappedTransposer<T: Transposer, P: SharedPointerKind> {
    pub transposer: T,
    pub metadata: TransposerMetaData<T, P>,
}

impl<T: Transposer, P: SharedPointerKind> Clone for WrappedTransposer<T, P>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            transposer: self.transposer.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> WrappedTransposer<T, P> {
    pub fn new(transposer: T, rng_seed: [u8; 32]) -> Self {
        let metadata = TransposerMetaData::new(rng_seed);

        Self {
            transposer,
            metadata,
        }
    }

    /// create a wrapped transposer, and perform all T::default scheduled events.
    pub async fn init(&mut self) {
        let mut context = InitUpdateContext::new(&mut self.metadata);

        self.transposer
            .init(InitContext::new_mut(&mut context))
            .await;
    }

    /// handle an input, and all scheduled events that occur at the same time.
    pub async fn handle_input<I: TransposerInput<Base = T>>(
        &mut self,
        time: T::Time,
        input: &I,
        input_event: &I::InputEvent,

        // mutable references must not be held over await points.
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) where
        T: TransposerInputEventHandler<I>,
    {
        let time = SubStepTime {
            index: self.metadata.last_updated.map(|x| x.index).unwrap_or(0) + 1,
            time,
        };

        let mut context = SubStepUpdateContext::new(time, &mut self.metadata, shared_step_state);
        self.transposer
            .handle_input_event(
                input,
                input_event,
                HandleInputContext::new_mut(&mut context),
            )
            .await;

        self.metadata.last_updated = Some(time);
    }

    /// handle all scheduled events occuring at `time` (if any)
    pub async fn handle_scheduled(
        &mut self,
        time: T::Time,

        // mutable references must not be held over await points.
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) {
        let mut time = SubStepTime {
            index: self.metadata.last_updated.map(|x| x.index).unwrap_or(0) + 1,
            time,
        };

        let mut context = SubStepUpdateContext::new(time, &mut self.metadata, shared_step_state);

        while context.metadata.get_next_scheduled_time().map(|s| s.time) == Some(time.time) {
            let (_, e) = context.metadata.pop_first_event().unwrap();
            self.transposer
                .handle_scheduled_event(e, HandleScheduleContext::new_mut(&mut context))
                .await;
            context.metadata.last_updated = Some(time);
            time.index += 1;
        }
    }
}
