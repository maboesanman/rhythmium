use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::{Deref, DerefMut},
    sync::Arc,
    task::Waker,
};

use futures::task::{waker, ArcWake};
use parking_lot::Mutex;

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

    pub fn get_source_interrupt_waker(&self, source_hash: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: SourceInterruptWakerData { source_hash },
            inner: self.inner.clone(),
        }))
    }

    pub fn get_source_channel_waker(&self, source_hash: u64, interpolation_uuid: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: SourceChannelWakerData {
                source_hash,
                interpolation_uuid,
            },
            inner: self.inner.clone(),
        }))
    }

    pub fn get_source_step_waker(&self, source_hash: u64, step_uuid: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: SourceStepWakerData {
                source_hash,
                step_uuid,
            },
            inner: self.inner.clone(),
        }))
    }

    pub fn get_step_waker(&self, step_uuid: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: StepWakerData { step_uuid },
            inner: self.inner.clone(),
        }))
    }

    pub fn get_interpolation_waker(&self, interpolation_uuid: u64) -> Waker {
        waker(Arc::new(WrappedWaker {
            data: InterpolationWakerData { interpolation_uuid },
            inner: self.inner.clone(),
        }))
    }
}

#[derive(Debug)]
enum TransposeInterruptWakerContainer {
    Working(Vec<DeferredWake>),
    Rest(TransposeInterruptWakerInner),
}

#[derive(Debug, Clone)]
enum DeferredWake {
    SourceInterrupt(SourceInterruptWakerData),
    SourceChannel(SourceChannelWakerData),
    Step(StepWakerData),
    SourceStep(SourceStepWakerData),
    Interpolation(InterpolationWakerData),
}

impl WakerData for DeferredWake {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        match self {
            DeferredWake::SourceInterrupt(d) => d.wake(inner),
            DeferredWake::SourceChannel(d) => d.wake(inner),
            DeferredWake::Step(d) => d.wake(inner),
            DeferredWake::SourceStep(d) => d.wake(inner),
            DeferredWake::Interpolation(d) => d.wake(inner),
        }
    }

    fn defer(&self) -> DeferredWake {
        self.clone()
    }
}

#[derive(Debug, Clone)]
pub struct TransposeInterruptWakerInner {
    pub state_interrupt_woken: VecDeque<u64 /* input_hash */>,
    pub state_interrupt_pending: VecDeque<u64 /* input_hash */>,

    pub step_item: Option<StepItem>,

    pub interrupt_waker: Waker,

    pub channels: HashMap<usize /* channel */, ChannelItem>,
}

impl TransposeInterruptWakerInner {
    fn new(input_hashes: impl Iterator<Item = u64>) -> Self {
        Self {
            state_interrupt_woken: input_hashes.collect(),
            state_interrupt_pending: VecDeque::new(),
            // need to mark interpolation as ready to poll so we bootstrap somewhere
            step_item: Some(StepItem {
                step_uuid: 0,
                step_woken: true,
                input_state_status: Status::None,
            }),
            interrupt_waker: Waker::noop().clone(),
            channels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepItem {
    pub step_uuid: u64,
    pub step_woken: bool,
    pub input_state_status: Status,
}

#[derive(Debug, Clone)]
pub struct ChannelItem {
    pub interpolation_uuid: u64,
    pub waker: Waker,
    pub interpolation_woken: bool,
    pub input_state_status: Status,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    Ready {
        input_hash: u64,
        input_channel: usize,
    },
    Pending {
        input_hash: u64,
        input_channel: usize,
    },
    None,
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
struct SourceInterruptWakerData {
    source_hash: u64,
}

impl WakerData for SourceInterruptWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        if let Some(pos) = inner
            .state_interrupt_pending
            .iter()
            .position(|&x| x == self.source_hash)
        {
            inner.state_interrupt_pending.remove(pos);
        } else if inner.state_interrupt_woken.contains(&self.source_hash) {
            return;
        }

        inner.state_interrupt_woken.push_back(self.source_hash);
        inner.interrupt_waker.wake_by_ref();
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::SourceInterrupt(*self)
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceChannelWakerData {
    interpolation_uuid: u64,
    source_hash: u64,
}

impl WakerData for SourceChannelWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        let channel_item = inner.channels.values_mut().find(|channel_item| {
            channel_item.interpolation_uuid == self.interpolation_uuid
                && match channel_item.input_state_status {
                    Status::Ready { .. } => false,
                    Status::Pending { input_hash, .. } => input_hash == self.source_hash,
                    Status::None => false,
                }
        });

        let channel_item = match channel_item {
            Some(c) => c,
            None => return,
        };

        match channel_item.input_state_status {
            Status::Pending {
                input_hash,
                input_channel,
            } => {
                channel_item.input_state_status = Status::Ready {
                    input_hash,
                    input_channel,
                };
                channel_item.waker.wake_by_ref();
            }
            _ => panic!(),
        };
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::SourceChannel(*self)
    }
}

#[derive(Debug, Clone, Copy)]
struct StepWakerData {
    step_uuid: u64,
}

impl WakerData for StepWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        let step_item = match &mut inner.step_item {
            Some(s) => s,
            None => return,
        };

        if step_item.step_uuid != self.step_uuid {
            return;
        }

        if step_item.step_woken {
            return;
        }

        step_item.step_woken = true;
        inner.interrupt_waker.wake_by_ref();
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::Step(*self)
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceStepWakerData {
    step_uuid: u64,
    source_hash: u64,
}

impl WakerData for SourceStepWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        let step_item = match &mut inner.step_item {
            Some(s) => s,
            None => return,
        };

        if step_item.step_uuid != self.step_uuid {
            return;
        }

        if let Status::Pending {
            input_hash,
            input_channel,
        } = step_item.input_state_status
        {
            if input_hash != self.source_hash {
                return;
            }

            step_item.input_state_status = Status::Ready {
                input_hash,
                input_channel,
            };
            inner.interrupt_waker.wake_by_ref();
        }
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::SourceStep(*self)
    }
}

#[derive(Debug, Clone, Copy)]
struct InterpolationWakerData {
    interpolation_uuid: u64,
}

impl WakerData for InterpolationWakerData {
    fn wake(&self, inner: &mut TransposeInterruptWakerInner) {
        let channel_item = inner.channels.values_mut().find(|channel_item| {
            channel_item.interpolation_uuid == self.interpolation_uuid
                && !channel_item.interpolation_woken
        });

        let channel_item = match channel_item {
            Some(c) => c,
            None => return,
        };

        channel_item.interpolation_woken = true;
        channel_item.waker.wake_by_ref();
    }

    fn defer(&self) -> DeferredWake {
        DeferredWake::Interpolation(*self)
    }
}
