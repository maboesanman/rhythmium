use std::{
    sync::{Arc, Weak, atomic::AtomicBool},
    task::Waker,
};

use futures_util::task::ArcWake;

pub struct WakeObserver(Arc<WakeObserverInner>);

struct WakeObserverInner {
    waker: Waker,
    woken: AtomicBool,
}

impl ArcWake for WakeObserverInner {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.waker.wake_by_ref();
        arc_self
            .woken
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl WakeObserver {
    pub fn new() -> Self {
        WakeObserver(Arc::new(WakeObserverInner {
            waker: futures::task::noop_waker(),
            woken: AtomicBool::new(false),
        }))
    }

    pub fn wrap_waker(&mut self, waker: Waker) -> Waker {
        let new_inner = WakeObserverInner {
            waker,
            woken: AtomicBool::new(false),
        };
        self.0 = Arc::new(new_inner);
        futures::task::waker(self.0.clone())
    }

    pub fn was_woken(&self) -> bool {
        self.0.woken.load(std::sync::atomic::Ordering::Acquire)
    }
}
