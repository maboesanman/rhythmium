use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
    task::Waker,
};

use futures::task::{ArcWake, waker};
use parking_lot::Mutex;

use crate::source::traits::SourceContext;

pub struct TransposeWakerObserver {
    inner: Arc<Mutex<TransposeInterruptWakerContainer>>,
}

pub struct InnerGuard {
    inner: Option<TransposeInterruptWakerInner>,
    container: Arc<Mutex<TransposeInterruptWakerContainer>>,
}

impl Deref for InnerGuard {
    type Target = TransposeInterruptWakerInner;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl DerefMut for InnerGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl Drop for InnerGuard {
    fn drop(&mut self) {
        let mut lock = self.container.lock();

        let deferred = match &mut *lock {
            TransposeInterruptWakerContainer::Working(deferred_wakes) => {
                core::mem::take(deferred_wakes)
            }
            TransposeInterruptWakerContainer::Rest(_) => Vec::new(),
        };

        for deferred_item in deferred {
            deferred_item.wake(self.inner.as_mut().unwrap());
        }

        *lock = TransposeInterruptWakerContainer::Rest(core::mem::take(&mut self.inner).unwrap())
    }
}

#[allow(dead_code)]
impl TransposeWakerObserver {
    pub fn new(input_hashes: impl Iterator<Item = u64>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TransposeInterruptWakerContainer::Rest(
                TransposeInterruptWakerInner::new(input_hashes),
            ))),
        }
    }

    pub fn lock(&self) -> InnerGuard {
        let mut lock = self.inner.lock();
        let inner = core::mem::replace(
            &mut *lock,
            TransposeInterruptWakerContainer::Working(Vec::new()),
        );
        let inner = match inner {
            TransposeInterruptWakerContainer::Rest(i) => i,
            _ => panic!(),
        };
        drop(lock);
        InnerGuard {
            inner: Some(inner),
            container: self.inner.clone(),
        }
    }

    pub fn get_waker_for_input_poll_interrupt(&self, input_hash: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: InputInterruptWakerData { input_hash },
            inner: self.inner.clone(),
        }))
    }

    pub fn get_waker_for_future_poll_from_step(&self, step_uuid: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: StepFutureWakerData { step_uuid },
            inner: self.inner.clone(),
        }))
    }

    pub fn get_context_for_input_poll_from_step(
        &self,
        input_hash: u64,
        step_id: u64,
    ) -> SourceContext {
        todo!()
    }

    pub fn get_waker_for_future_poll_from_interpolation(&self, interpolation_uuid: u64) -> Waker {
        todo!()
    }

    pub fn get_context_for_input_poll_from_interpolation(
        &self,
        input_hash: u64,
        interpolation_uuid: u64,
        channel: u64,
    ) -> SourceContext {
        todo!()
    }
}

#[derive(Debug)]
enum TransposeInterruptWakerContainer {
    Working(Vec<DeferredWake>),
    Rest(TransposeInterruptWakerInner),
}

#[derive(Debug, Clone)]
enum DeferredWake {
    SourceInterrupt(InputInterruptWakerData),
    // SourceChannel(SourceChannelWakerData),
    Step(StepFutureWakerData),
    // SourceStep(SourceStepWakerData),
    // Interpolation(InterpolationWakerData),
}

impl WakerData for DeferredWake {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        match self {
            DeferredWake::SourceInterrupt(d) => d.wake(inner),
            // DeferredWake::SourceChannel(d) => d.wake(inner),
            DeferredWake::Step(d) => d.wake(inner),
            // DeferredWake::SourceStep(d) => d.wake(inner),
            // DeferredWake::Interpolation(d) => d.wake(inner),
        }
    }

    fn defer(&self) -> DeferredWake {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct TransposeInterruptWakerInner {
    /// the latest interrupt waker, updated on every call to the source which has a waker.
    pub interrupt_waker: Waker,

    /// the list of inputs whose interrupt wakers have been invoked.
    pub input_interrupt_woken: VecDeque<u64 /* input_hash */>,

    /// the list of inputs whose interrupts are pending but not yet woken.
    pub input_interrupt_pending: VecDeque<u64 /* input_hash */>,

    /// the active step item if any, that is being polled.
    /// this holds metadata about the saturation future and the input state polls
    /// for any state requests.
    pub step_interrupt: Option<StepStatus>,
    // /// the channel usage information for input channels.
    // pub channels: HashMap<usize /* channel */, ChannelItem>,
}

#[derive(Debug, Clone)]
pub struct StepStatus {
    pub step_uuid: u64,
    pub step_saturation_future_status: FutureStatus,
    pub input_state_status: InputStateStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutureStatus {
    Uninitialized,
    Woken,
    Pending,
}

#[derive(Debug, Clone)]
pub struct ChannelItem {
    pub interpolation_uuid: u64,
    pub waker: Waker,
    pub interpolation_status: FutureStatus,
    pub input_state_status: InputStateStatus,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InputStateStatus {
    Woken {
        input_hash: u64,
        input_channel: usize,
    },
    Pending {
        input_hash: u64,
        input_channel: usize,
    },
    None,
}

impl TransposeInterruptWakerInner {
    fn new(input_hashes: impl Iterator<Item = u64>) -> Self {
        Self {
            interrupt_waker: Waker::noop().clone(),
            input_interrupt_woken: input_hashes.collect(),
            input_interrupt_pending: VecDeque::new(),
            // need to mark interpolation as ready to poll so we bootstrap somewhere
            step_interrupt: Some(StepStatus {
                step_uuid: 0,
                step_saturation_future_status: FutureStatus::Uninitialized,
                input_state_status: InputStateStatus::None,
            }),
            // channels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct WrappedWaker<D> {
    data: D,
    inner: Arc<Mutex<TransposeInterruptWakerContainer>>,
}

trait WakerData: Send + Sync {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner);

    fn defer(&self) -> DeferredWake;
}

impl<D: WakerData> ArcWake for WrappedWaker<D> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut container = arc_self.inner.lock();
        match &mut *container {
            TransposeInterruptWakerContainer::Working(items) => {
                items.push(arc_self.data.defer());
            }
            TransposeInterruptWakerContainer::Rest(inner) => {
                arc_self.data.wake(inner);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct InputInterruptWakerData {
    input_hash: u64,
}

impl WakerData for InputInterruptWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        if inner.input_interrupt_woken.contains(&self.input_hash) {
            return;
        }

        if let Some(pos) = inner
            .input_interrupt_pending
            .iter()
            .position(|&x| x == self.input_hash)
        {
            inner.input_interrupt_pending.swap_remove_back(pos);
        }

        inner.input_interrupt_woken.push_back(self.input_hash);
        inner.interrupt_waker.wake_by_ref();
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::SourceInterrupt(*self)
    }
}

#[derive(Debug, Clone, Copy)]
struct StepFutureWakerData {
    step_uuid: u64,
}

impl WakerData for StepFutureWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        let step_item = match &mut inner.step_interrupt {
            Some(s) => s,
            None => return,
        };

        if step_item.step_uuid != self.step_uuid {
            return;
        }

        if step_item.step_saturation_future_status != FutureStatus::Pending {
            return;
        }

        step_item.step_saturation_future_status = FutureStatus::Woken;
        inner.interrupt_waker.wake_by_ref();
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::Step(*self)
    }
}

// struct StepInputStateWakerData {
//     step_uuid: u64,
//     input_uuid: u64,
// }

// #[derive(Debug, Clone, Copy)]
// struct SourceChannelWakerData {
//     interpolation_uuid: u64,
//     source_hash: u64,
// }

// impl WakerData for SourceChannelWakerData {
//     fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
//         let channel_item = inner.channels.values_mut().find(|channel_item| {
//             channel_item.interpolation_uuid == self.interpolation_uuid
//                 && match channel_item.input_state_status {
//                     InputStateStatus::Woken { .. } => false,
//                     InputStateStatus::Pending { input_hash, .. } => input_hash == self.source_hash,
//                     InputStateStatus::None => false,
//                 }
//         });

//         let channel_item = match channel_item {
//             Some(c) => c,
//             None => return,
//         };

//         match channel_item.input_state_status {
//             InputStateStatus::Pending {
//                 input_hash,
//                 input_channel,
//             } => {
//                 channel_item.input_state_status = InputStateStatus::Woken {
//                     input_hash,
//                     input_channel,
//                 };
//                 channel_item.waker.wake_by_ref();
//             }
//             _ => panic!(),
//         };
//     }

//     fn defer(&self) -> DeferredWake {
//         DeferredWake::SourceChannel(*self)
//     }
// }

// #[derive(Debug, Clone, Copy)]
// struct SourceStepWakerData {
//     step_uuid: u64,
//     source_hash: u64,
// }

// impl WakerData for SourceStepWakerData {
//     fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
//         let step_item = match &mut inner.step_interrupt {
//             Some(s) => s,
//             None => return,
//         };

//         if step_item.step_uuid != self.step_uuid {
//             return;
//         }

//         if let InputStateStatus::Pending {
//             input_hash,
//             input_channel,
//         } = step_item.input_state_status
//         {
//             if input_hash != self.source_hash {
//                 return;
//             }

//             step_item.input_state_status = InputStateStatus::Woken {
//                 input_hash,
//                 input_channel,
//             };
//             inner.interrupt_waker.wake_by_ref();
//         }
//     }

//     fn defer(&self) -> DeferredWake {
//         DeferredWake::SourceStep(*self)
//     }
// }

// #[derive(Debug, Clone, Copy)]
// struct InterpolationWakerData {
//     interpolation_uuid: u64,
// }

// impl WakerData for InterpolationWakerData {
//     fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
//         let channel_item = inner.channels.values_mut().find(|channel_item| {
//             channel_item.interpolation_uuid == self.interpolation_uuid
//                 && (channel_item.interpolation_status != FutureStatus::Woken)
//         });

//         let channel_item = match channel_item {
//             Some(c) => c,
//             None => return,
//         };

//         channel_item.interpolation_status = FutureStatus::Woken;
//         channel_item.waker.wake_by_ref();
//     }

//     fn defer(&self) -> DeferredWake {
//         DeferredWake::Interpolation(*self)
//     }
// }
