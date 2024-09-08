use std::{any::TypeId, cmp::Ordering, future::Future, marker::PhantomPinned, ptr};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{step::wrapped_transposer::WrappedTransposer, Transposer, TransposerInput};

use super::SubStep;


struct InputSubStepData<T: Transposer, I: TransposerInput<Base = T>> {
    input: I,
    input_event: I::InputEvent,
}

enum InputSubStepStatus<T: Transposer, P: SharedPointerKind, Fut> {
    Unsaturated {
        time: T::Time,
    },
    Saturating {
        time: T::Time,
        future: Fut,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>
    },
}

struct InputSubStep<T: Transposer, P: SharedPointerKind, I: TransposerInput<Base = T>, Fut> {
    status: InputSubStepStatus<T, P, Fut>,
    data: InputSubStepData<T, I>,
    _pinned: PhantomPinned,
}

impl<T, P, I, Fut> SubStep<T, P> for InputSubStep<T, P, I, Fut>
where
    T: Transposer,
    P: SharedPointerKind,
    I: TransposerInput<Base = T>,
    Fut: Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>,
{
    fn is_input(&self) -> bool {
        true
    }

    fn input_sort(&self) -> Option<(u64, TypeId)> {
        Some((I::SORT, TypeId::of::<I>()))
    }

    fn is_unsaturated(&self) -> bool {
        matches!(self.status, InputSubStepStatus::Unsaturated { .. })
    }

    fn is_saturating(&self) -> bool {
        matches!(self.status, InputSubStepStatus::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self.status, InputSubStepStatus::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match &self.status {
            InputSubStepStatus::Unsaturated { time } => *time,
            InputSubStepStatus::Saturating { time, .. } => *time,
            InputSubStepStatus::Saturated { wrapped_transposer } => wrapped_transposer.metadata.last_updated.time,
        }
    }

    fn cmp(&self, other: &dyn SubStep<T, P>) -> Ordering {
        match self.get_time().cmp(&other.get_time()) {
            Ordering::Equal => {}
            ne => return ne,
        };

        if other.is_init() {
            return Ordering::Greater;
        }

        if other.is_scheduled() {
            return Ordering::Less;
        }

        match self.input_sort().cmp(&other.input_sort()) {
            Ordering::Equal => {}
            ne => return ne,
        }

        let other_addr = (other as *const dyn SubStep<T, P>).addr();
        let other_ptr = (self as *const Self).with_addr(other_addr);
        let other = unsafe { &*other_ptr };

        match self.data.input.cmp(&other.data.input) {
            Ordering::Equal => {}
            ne => return ne,
        }

        self.data.input_event.cmp(&other.data.input_event)
    }
}