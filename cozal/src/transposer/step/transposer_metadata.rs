use archery::SharedPointerKind;
use rand::SeedableRng;
use rand_chacha::ChaCha12Core;
use rand_chacha::rand_core::block::BlockRng;

use super::expire_handle_factory::ExpireHandleFactory;
use super::time::{ScheduledTime, SubStepTime};
use crate::transposer::Transposer;
use crate::transposer::context::ExpireEventError;
use crate::transposer::expire_handle::ExpireHandle;

/// TransposerMetaData is a struct that holds all the data that is needed to run a transposer, besides
/// the actual transposer struct. Essentially this is all the stuff that the various Contexts provide
/// access to, apart from inputs.
pub struct TransposerMetaData<T: Transposer, P: SharedPointerKind> {
    /// this has index 0 while processing inputs, which is technically wrong, but should never be accessible.
    pub last_updated: Option<SubStepTime<T::Time>>,

    /// A collection of (time, payload) for each scheduled event, indexed by time.
    pub schedule: rpds::RedBlackTreeMap<ScheduledTime<T::Time>, T::Scheduled, P>,

    /// A map of ExpireHandle->ScheduledTime (kept in sync with backward)
    pub expire_handles_forward: rpds::HashTrieMap<ExpireHandle, ScheduledTime<T::Time>, P>,

    /// A map of ScheduledTime->ExpireHandle (kept in sync with forward)
    pub expire_handles_backward: rpds::RedBlackTreeMap<ScheduledTime<T::Time>, ExpireHandle, P>,

    /// The source of new ExpireHandles
    pub expire_handle_factory: ExpireHandleFactory,

    /// The deterministic source of entropy.
    pub rng: BlockRng<ChaCha12Core>,
}

impl<T: Transposer, P: SharedPointerKind> Clone for TransposerMetaData<T, P> {
    fn clone(&self) -> Self {
        Self {
            last_updated: self.last_updated,
            schedule: self.schedule.clone(),
            expire_handles_forward: self.expire_handles_forward.clone(),
            expire_handles_backward: self.expire_handles_backward.clone(),
            expire_handle_factory: self.expire_handle_factory.clone(),
            rng: self.rng.clone(),
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> TransposerMetaData<T, P> {
    pub fn new(rng_seed: [u8; 32]) -> Self {
        let schedule = rpds::RedBlackTreeMap::new_with_ptr_kind();
        let expire_handles_forward = rpds::HashTrieMap::new_with_hasher_and_ptr_kind(
            std::collections::hash_map::RandomState::default(),
        );
        let expire_handles_backward = rpds::RedBlackTreeMap::new_with_ptr_kind();

        Self {
            last_updated: None,
            schedule,
            expire_handles_forward,
            expire_handles_backward,
            expire_handle_factory: ExpireHandleFactory::default(),
            rng: BlockRng::new(ChaCha12Core::from_seed(rng_seed)),
        }
    }

    pub fn schedule_event(&mut self, time: ScheduledTime<T::Time>, payload: T::Scheduled) {
        self.schedule.insert_mut(time, payload);
    }

    pub fn schedule_event_expireable(
        &mut self,
        time: ScheduledTime<T::Time>,
        payload: T::Scheduled,
    ) -> ExpireHandle {
        self.schedule_event(time, payload);

        let handle = self.expire_handle_factory.next();
        self.expire_handles_forward.insert_mut(handle, time);
        self.expire_handles_backward.insert_mut(time, handle);

        handle
    }

    pub fn expire_event(
        &mut self,
        handle: ExpireHandle,
    ) -> Result<(T::Time, T::Scheduled), ExpireEventError> {
        match self.expire_handles_forward.get(&handle) {
            Some(time) => {
                let t = time.time;

                let payload = self.schedule.get(time).unwrap().clone();

                // maps are kept in sync
                self.schedule.remove_mut(time);
                self.expire_handles_backward.remove_mut(time);
                self.expire_handles_forward.remove_mut(&handle);

                Ok((t, payload))
            }
            None => Err(ExpireEventError::InvalidOrUsedHandle),
        }
    }

    pub fn get_next_scheduled_time(&self) -> Option<&ScheduledTime<T::Time>> {
        self.schedule.first().map(|(k, _)| k)
    }

    pub fn pop_first_event(&mut self) -> Option<(ScheduledTime<T::Time>, T::Scheduled)> {
        let (k, v) = self.schedule.first()?;
        let k = *k;
        let v = v.clone();

        self.schedule.remove_mut(&k);

        if let Some(h) = self.expire_handles_backward.get(&k) {
            let h = *h;
            self.expire_handles_backward.remove_mut(&k);
            self.expire_handles_forward.remove_mut(&h);
        }

        Some((k, v))
    }
}
