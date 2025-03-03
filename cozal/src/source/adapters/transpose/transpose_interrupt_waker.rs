use std::{collections::{HashMap, VecDeque}, ops::DerefMut, sync::Arc, task::Waker};

use futures::task::{waker, ArcWake};
use parking_lot::{Mutex, MutexGuard};

use crate::source::traits::SourceContext;


pub struct TransposeWakerObserver {
    inner: Arc<Mutex<TransposeInterruptWakerInner>>,
}

impl TransposeWakerObserver {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TransposeInterruptWakerInner::new()))
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, TransposeInterruptWakerInner> {
        self.inner.lock()
    }

    pub fn get_source_interrupt_waker(&self, source_hash: u64) -> Waker {
        waker(Arc::new(SourceInterruptWaker {
            source_hash,
            inner: self.inner.clone()
        }))
    }

    pub fn get_source_channel_waker(&self, source_hash: u64, interpolation_uuid: u64) -> Waker {
        waker(Arc::new(SourceChannelWaker {
            source_hash,
            interpolation_uuid,
            inner: self.inner.clone(),
        }))
    }

    pub fn get_source_step_waker(&self, source_hash: u64, step_uuid: u64) -> Waker {
        waker(Arc::new(SourceStepWaker {
            source_hash,
            step_uuid,
            inner: self.inner.clone(),
        }))
    }

    pub fn get_step_waker(&self, step_uuid: u64) -> Waker {
        waker(Arc::new(StepWaker {
            step_uuid,
            inner: self.inner.clone(),
        }))
    }

    pub fn get_interpolation_waker(&self, interpolation_uuid: u64) -> Waker {
        waker(Arc::new(InterpolationWaker {
            interpolation_uuid,
            inner: self.inner.clone(),
        }))
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
    fn new() -> Self {
        Self {
            state_interrupt_woken: VecDeque::new(),
            state_interrupt_pending: VecDeque::new(),
            step_item: None,
            interrupt_waker: Waker::noop().clone(),
            channels: HashMap::new(),
        }
    }

    pub fn register_context(&mut self, cx: &SourceContext) {
        self.interrupt_waker = cx.interrupt_waker.clone();
        // self.channels.get_mut(&cx.channel)
        todo!()
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
    None
}

#[derive(Debug, Clone)]
struct SourceInterruptWaker {
    source_hash: u64,
    inner: Arc<Mutex<TransposeInterruptWakerInner>>,
}

impl ArcWake for SourceInterruptWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut inner = arc_self.inner.lock();

        if let Some(pos) = inner.state_interrupt_pending.iter().position(|&x| x == arc_self.source_hash) {
            inner.state_interrupt_pending.remove(pos);
        } else if inner.state_interrupt_woken.contains(&arc_self.source_hash) {
            return;
        }

        inner.state_interrupt_woken.push_back(arc_self.source_hash);
        inner.interrupt_waker.wake_by_ref();
    }
}

#[derive(Debug, Clone)]
struct SourceChannelWaker {
    interpolation_uuid: u64,
    source_hash: u64,
    inner: Arc<Mutex<TransposeInterruptWakerInner>>,
}

impl ArcWake for SourceChannelWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut inner = arc_self.inner.lock();

        let channel_item = inner.channels.values_mut().find(|channel_item| {
            channel_item.interpolation_uuid == arc_self.interpolation_uuid
            && match channel_item.input_state_status {
                Status::Ready { .. } => false,
                Status::Pending { input_hash, .. } => input_hash == arc_self.source_hash,
                Status::None => false,
            }
    });

        let channel_item = match channel_item {
            Some(c) => c,
            None => return,
        };

        match channel_item.input_state_status {
            Status::Pending { input_hash, input_channel } => {
                channel_item.input_state_status = Status::Ready { input_hash, input_channel };
                channel_item.waker.wake_by_ref();
            },
            _ => panic!()
        };

    }
}

#[derive(Debug, Clone)]
struct StepWaker {
    step_uuid: u64,
    inner: Arc<Mutex<TransposeInterruptWakerInner>>,
}

impl ArcWake for StepWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut inner = arc_self.inner.lock();

        let step_item = match &mut inner.step_item {
            Some(s) => s,
            None => return,
        };

        if step_item.step_uuid != arc_self.step_uuid {
            return;
        }

        if step_item.step_woken {
            return;
        }

        step_item.step_woken = true;
        inner.interrupt_waker.wake_by_ref();
    }
}

#[derive(Debug, Clone)]
struct SourceStepWaker {
    step_uuid: u64,
    source_hash: u64,
    inner: Arc<Mutex<TransposeInterruptWakerInner>>,
}

impl ArcWake for SourceStepWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut inner = arc_self.inner.lock();

        let step_item = match &mut inner.step_item {
            Some(s) => s,
            None => return,
        };

        if step_item.step_uuid != arc_self.step_uuid {
            return;
        }

        if let Status::Pending { input_hash, input_channel } = step_item.input_state_status {
            if input_hash != arc_self.source_hash {
                return
            }

            step_item.input_state_status = Status::Ready { input_hash, input_channel };
            inner.interrupt_waker.wake_by_ref();
        }
    }
}

#[derive(Debug, Clone)]
struct InterpolationWaker {
    interpolation_uuid: u64,
    inner: Arc<Mutex<TransposeInterruptWakerInner>>,
}

impl ArcWake for InterpolationWaker {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let mut inner = arc_self.inner.lock();

        let channel_item = inner.channels.values_mut().find(|channel_item| {
            channel_item.interpolation_uuid == arc_self.interpolation_uuid
            && !channel_item.interpolation_woken
        });

        let channel_item = match channel_item {
            Some(c) => c,
            None => return,
        };

        channel_item.interpolation_woken = true;
        channel_item.waker.wake_by_ref();
    }
}