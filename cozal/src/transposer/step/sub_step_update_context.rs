use std::ptr::NonNull;

use archery::SharedPointerKind;
use rand_chacha::rand_core::CryptoRngCore;

use super::time::SubStepTime;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::*;
use crate::transposer::expire_handle::ExpireHandle;
use crate::transposer::input_state_manager::InputStateManager;
use crate::transposer::output_event_manager::OutputEventManager;
use crate::transposer::Transposer;

/// This is the interface through which you can do a variety of functions in your transposer.
///
/// the primary features are scheduling and expiring events,
/// though there are more methods to interact with the engine.
pub struct SubStepUpdateContext<'update, T: Transposer, P: SharedPointerKind> {
    time: SubStepTime<T::Time>,
    // these are pointers because this is stored next to the targets.
    pub metadata: &'update mut TransposerMetaData<T, P>,

    current_emission_index: usize,

    // the ownership of this is effectively shared up a couple levels in the step, so we can't
    // store live references to it.
    shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
}

impl<'update, T: Transposer, P: SharedPointerKind> InitContext<'update, T>
    for SubStepUpdateContext<'update, T, P>
{
}
impl<'update, T: Transposer, P: SharedPointerKind> HandleInputContext<'update, T>
    for SubStepUpdateContext<'update, T, P>
{
}
impl<'update, T: Transposer, P: SharedPointerKind> HandleScheduleContext<'update, T>
    for SubStepUpdateContext<'update, T, P>
{
}
impl<'update, T: Transposer, P: SharedPointerKind> SubStepUpdateContext<'update, T, P> {
    // SAFETY: need to gurantee the metadata pointer outlives this object.
    pub fn new(
        time: SubStepTime<T::Time>,
        metadata: &'update mut TransposerMetaData<T, P>,
        shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    ) -> Self {
        Self {
            time,
            metadata,
            current_emission_index: 0,
            shared_step_state,
        }
    }

    unsafe fn get_shared_step_state(
        &mut self,
    ) -> &mut (OutputEventManager<T>, InputStateManager<T>) {
        let mut shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        unsafe { shared_step_state.as_mut() }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> InputStateManagerContext<'update, T>
    for SubStepUpdateContext<'update, T, P>
{
    fn get_input_state_manager(&mut self) -> NonNull<InputStateManager<T>> {
        let mut shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        NonNull::from(unsafe { &mut shared_step_state.as_mut().1 })
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> OutputEventManagerContext<T>
    for SubStepUpdateContext<'update, T, P>
{
    fn get_output_event_manager(&mut self) -> NonNull<OutputEventManager<T>> {
        let mut shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        NonNull::from(unsafe { &mut shared_step_state.as_mut().0 })
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> ScheduleEventContext<T>
    for SubStepUpdateContext<'update, T, P>
{
    fn schedule_event(
        &mut self,
        time: T::Time,
        payload: T::Scheduled,
    ) -> Result<(), ScheduleEventError> {
        if time < self.time.time {
            return Err(ScheduleEventError::NewEventBeforeCurrent);
        }

        let time = self.time.spawn_scheduled(time, self.current_emission_index);

        self.metadata.schedule_event(time, payload);
        self.current_emission_index += 1;

        Ok(())
    }

    fn schedule_event_expireable(
        &mut self,
        time: T::Time,
        payload: T::Scheduled,
    ) -> Result<ExpireHandle, ScheduleEventError> {
        if time < self.time.time {
            return Err(ScheduleEventError::NewEventBeforeCurrent);
        }

        let time = self.time.spawn_scheduled(time, self.current_emission_index);

        let handle = self.metadata.schedule_event_expireable(time, payload);
        self.current_emission_index += 1;

        Ok(handle)
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> ExpireEventContext<T>
    for SubStepUpdateContext<'update, T, P>
{
    fn expire_event(
        &mut self,
        handle: ExpireHandle,
    ) -> Result<(T::Time, T::Scheduled), ExpireEventError> {
        self.metadata.expire_event(handle)
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> RngContext
    for SubStepUpdateContext<'update, T, P>
{
    fn get_rng(&mut self) -> &mut dyn CryptoRngCore {
        &mut self.metadata.rng
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> CurrentTimeContext<T>
    for SubStepUpdateContext<'update, T, P>
{
    fn current_time(&self) -> <T as Transposer>::Time {
        self.time.time
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> LastUpdatedTimeContext<T>
    for SubStepUpdateContext<'update, T, P>
{
    fn last_updated_time(&self) -> <T as Transposer>::Time {
        self.metadata.last_updated.time
    }
}
