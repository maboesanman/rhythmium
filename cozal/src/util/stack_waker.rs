use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::task::{Wake, Waker};

use parking_lot::Mutex;

#[derive(Debug)]
pub struct StackWaker {
    inner: Mutex<StackWakerInner>,
}

#[derive(Debug)]
struct StackWakerInner {
    wakers: HashMap<usize, Waker>,
}

impl StackWaker {
    pub fn new_empty() -> Weak<Self> {
        Weak::new()
    }

    // return whether or not this waker should be used to poll right away
    pub fn register(weak: &mut Weak<Self>, key: usize, waker: Waker) -> Option<Waker> {
        let arc_self = match weak.upgrade() {
            Some(a) => a,
            None => Arc::new(StackWaker {
                inner: Mutex::new(StackWakerInner {
                    wakers: HashMap::with_capacity(1),
                }),
            }), // this means the waker was dropped; need a new one.
        };

        let new_weak = Arc::downgrade(&arc_self);
        *weak = new_weak;

        let mut lock = arc_self.inner.lock();
        let emit_waker = lock.wakers.is_empty();
        let _ = lock.wakers.insert(key, waker);

        if !emit_waker {
            return None; // no need for a waker, we weren't woken.
        }

        let new_waker = arc_self.clone();
        Some(new_waker.into())
    }

    pub fn un_register(weak: &mut Weak<Self>, key: usize) {
        let arc_self = match weak.upgrade() {
            Some(a) => a,
            None => return, // it was dropped, won't be woken anyway
        };

        let mut lock = arc_self.inner.lock();
        lock.wakers.remove(&key);
    }
}

impl Wake for StackWaker {
    fn wake(self: Arc<Self>) {
        let mut wakers: Vec<_> = self.inner.lock().wakers.drain().map(|(_k, w)| w).collect();

        // try our best not to wake everything a million times...
        // wakers.sort_by_key(|w| {
        //     (
        //         w.as_raw().data() as usize,
        //         w.as_raw().vtable() as *const _ as usize,
        //     )
        // });
        wakers.dedup_by(|w1, w2| w1.will_wake(w2));

        for w in wakers.into_iter() {
            w.wake()
        }
    }
}
