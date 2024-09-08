use core::future::Future;
use core::pin::Pin;
use std::borrow::BorrowMut;
use std::cell::{RefCell, UnsafeCell};
use std::ptr::NonNull;
use std::rc::Rc;

use archery::SharedPointerKind;
use rand_chacha::rand_core::CryptoRngCore;

use super::time::SubStepTime;
use super::transposer_metadata::TransposerMetaData;
use super::InputState;
use crate::transposer::context::*;
use crate::transposer::expire_handle::ExpireHandle;
use crate::transposer::Transposer;

/// This is the interface through which you can do a variety of functions in your transposer.
///
/// the primary features are scheduling and expiring events,
/// though there are more methods to interact with the engine.
pub struct SubStepUpdateContext<'update, T: Transposer, P: SharedPointerKind, Is> {
    time: SubStepTime<T::Time>,
    // these are pointers because this is stored next to the targets.
    pub metadata: &'update mut TransposerMetaData<T, P>,

    // pub time:               SubStepTime<T::Time>,
    pub outputs_to_swallow: usize,
    current_emission_index: usize,

    // the ownership of this is effectively shared up a couple levels in the step, so we can't
    // store live references to it.
    shared_step_state: NonNull<UnsafeCell<Is>>,
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> InitContext<'update, T>
    for SubStepUpdateContext<'update, T, P, Is>
{
}
impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> HandleInputContext<'update, T>
    for SubStepUpdateContext<'update, T, P, Is>
{
}
impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> HandleScheduleContext<'update, T>
    for SubStepUpdateContext<'update, T, P, Is>
{
}
impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> SubStepUpdateContext<'update, T, P, Is> {
    // SAFETY: need to gurantee the metadata pointer outlives this object.
    pub fn new(
        time: SubStepTime<T::Time>,
        metadata: &'update mut TransposerMetaData<T, P>,
        shared_step_state: NonNull<UnsafeCell<Is>>,
        outputs_to_swallow: usize,
    ) -> Self {
        Self {
            time,
            metadata,
            current_emission_index: 0,
            outputs_to_swallow,
            shared_step_state,
        }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> InputStateManagerContext<'update, T>
    for SubStepUpdateContext<'update, T, P, Is>
{
    fn get_input_state_manager(&mut self) -> &mut T::InputStateManager<'update> {
        let input_state: NonNull<UnsafeCell<Is>> = self.shared_step_state;
        let input_state: &UnsafeCell<Is> = unsafe { input_state.as_ref() };
        let input_state: *mut Is = input_state.get();
        let input_state: &mut Is = unsafe { input_state.as_mut() }.unwrap();

        let input_state_manager: &mut T::InputStateManager<'static> = input_state.get_provider();
        let input_state_manager: &mut T::InputStateManager<'update> = unsafe { core::mem::transmute(input_state_manager) };

        input_state_manager
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> ScheduleEventContext<T>
    for SubStepUpdateContext<'update, T, P, Is>
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

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> ExpireEventContext<T>
    for SubStepUpdateContext<'update, T, P, Is>
{
    fn expire_event(
        &mut self,
        handle: ExpireHandle,
    ) -> Result<(T::Time, T::Scheduled), ExpireEventError> {
        self.metadata.expire_event(handle)
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> EmitEventContext<T>
    for SubStepUpdateContext<'update, T, P, Is>
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

        // let (send, recv) = futures_channel::oneshot::channel();
        // self.output_sender.try_send((payload, send)).unwrap();

        // Box::pin(async move {
        //     recv.await.unwrap();
        // })

        todo!()
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> RngContext
    for SubStepUpdateContext<'update, T, P, Is>
{
    fn get_rng(&mut self) -> &mut dyn CryptoRngCore {
        &mut self.metadata.rng
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> CurrentTimeContext<T>
    for SubStepUpdateContext<'update, T, P, Is>
{
    fn current_time(&self) -> <T as Transposer>::Time {
        self.time.time
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> LastUpdatedTimeContext<T>
    for SubStepUpdateContext<'update, T, P, Is>
{
    fn last_updated_time(&self) -> <T as Transposer>::Time {
        self.metadata.last_updated.time
    }
}
