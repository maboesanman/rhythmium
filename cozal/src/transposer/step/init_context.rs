use archery::SharedPointerKind;
use rand_chacha::rand_core::CryptoRngCore;

use super::time::ScheduledTime;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::*;
use crate::transposer::expire_handle::ExpireHandle;
use crate::transposer::Transposer;

/// This is the interface through which you can do a variety of functions in your transposer.
///
/// the primary features are scheduling and expiring events,
/// though there are more methods to interact with the engine.
pub struct InitUpdateContext<'update, T: Transposer, P: SharedPointerKind> {
    // these are pointers because this is stored next to the targets.
    pub metadata: &'update mut TransposerMetaData<T, P>,

    current_emission_index: usize,
}

impl<'update, T: Transposer, P: SharedPointerKind> InitContextInner<'update, T>
    for InitUpdateContext<'update, T, P>
{
}

impl<'update, T: Transposer, P: SharedPointerKind> InitUpdateContext<'update, T, P> {
    // SAFETY: need to gurantee the metadata pointer outlives this object.
    pub fn new(metadata: &'update mut TransposerMetaData<T, P>) -> Self {
        Self {
            metadata,
            current_emission_index: 0,
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> ScheduleEventContextInfallible<T>
    for InitUpdateContext<'_, T, P>
{
    fn schedule_event(&mut self, time: T::Time, payload: T::Scheduled) {
        let time = ScheduledTime::init_spawn_scheduled(time, self.current_emission_index);

        self.metadata.schedule_event(time, payload);
        self.current_emission_index += 1;
    }

    fn schedule_event_expireable(&mut self, time: T::Time, payload: T::Scheduled) -> ExpireHandle {
        let time = ScheduledTime::init_spawn_scheduled(time, self.current_emission_index);

        let handle = self.metadata.schedule_event_expireable(time, payload);
        self.current_emission_index += 1;

        handle
    }
}

impl<T: Transposer, P: SharedPointerKind> RngContext for InitUpdateContext<'_, T, P> {
    fn get_rng(&mut self) -> &mut dyn CryptoRngCore {
        &mut self.metadata.rng
    }
}
