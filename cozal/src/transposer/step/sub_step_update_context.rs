use core::future::Future;
use core::pin::Pin;

use archery::SharedPointerKind;
use rand_chacha::rand_core::CryptoRngCore;

use super::time::SubStepTime;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::*;
use crate::transposer::expire_handle::ExpireHandle;
use crate::transposer::Transposer;

/// This is the interface through which you can do a variety of functions in your transposer.
///
/// the primary features are scheduling and expiring events,
/// though there are more methods to interact with the engine.
pub struct SubStepUpdateContext<'update, T: Transposer, P: SharedPointerKind> {
    time: SubStepTime<T::Time>,
    // these are pointers because this is stored next to the targets.
    pub metadata: &'update mut TransposerMetaData<T, P>,

    // pub time:               SubStepTime<T::Time>,
    pub outputs_to_swallow: usize,
    current_emission_index: usize,

    // values to output
    pub output_sender:
        futures_channel::mpsc::Sender<(T::OutputEvent, futures_channel::oneshot::Sender<()>)>,

    input_state: &'update T::InputStateManager,
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
        input_state: &'update T::InputStateManager,
        outputs_to_swallow: usize,
        output_sender: futures_channel::mpsc::Sender<(
            T::OutputEvent,
            futures_channel::oneshot::Sender<()>,
        )>,
    ) -> Self {
        Self {
            time,
            metadata,
            input_state,
            current_emission_index: 0,
            outputs_to_swallow,
            output_sender,
        }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> InputStateContextRaw<'update, T>
    for SubStepUpdateContext<'update, T, P>
{
    fn get_input_state_manager(&mut self) -> &'update T::InputStateManager {
        self.input_state
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

impl<'update, T: Transposer, P: SharedPointerKind> EmitEventContext<T>
    for SubStepUpdateContext<'update, T, P>
{
    fn emit_event(
        &mut self,
        payload: <T as Transposer>::OutputEvent,
    ) -> Pin<Box<dyn '_ + Future<Output = ()>>> {
        // if we need to swallow events still
        if self.outputs_to_swallow > 0 {
            self.outputs_to_swallow -= 1;
            return Box::pin(core::future::ready(()));
        }

        let (send, recv) = futures_channel::oneshot::channel();
        self.output_sender.try_send((payload, send)).unwrap();

        Box::pin(async move {
            recv.await.unwrap();
        })
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
