use std::{any::Any, cell::UnsafeCell, future::Future, ptr::NonNull};

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{step::{wrapped_transposer::WrappedTransposer, InputState}, Transposer};

use super::SubStep;


enum InitSubStepStatus<T: Transposer, P: SharedPointerKind, Fut> {
    Saturating {
        start_time: T::Time,
        future: Fut,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>
    },
}

impl<T: Transposer, P: SharedPointerKind, Fut: Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>> SubStep<T, P> for InitSubStepStatus<T, P, Fut> {
    fn is_init(&self) -> bool {
        true
    }

    fn is_unsaturated(&self) -> bool {
        false
    }

    fn is_saturating(&self) -> bool {
        matches!(self, InitSubStepStatus::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, InitSubStepStatus::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match self {
            InitSubStepStatus::Saturating { start_time, .. } => *start_time,
            InitSubStepStatus::Saturated { wrapped_transposer } => wrapped_transposer.metadata.last_updated.time,
        }
    }

    fn cmp(&self, other: &dyn SubStep<T, P>) -> std::cmp::Ordering {
        match other.is_init() {
            true => std::cmp::Ordering::Equal,
            false => std::cmp::Ordering::Less,
        }
    }
}

fn new_init_sub_step_internal<T: Transposer, P: SharedPointerKind, S: InputState<T>>(
    transposer: T,
    rng_seed: [u8; 32],
    start_time: T::Time,
    shared_step_state: NonNull<UnsafeCell<S>>,
) -> InitSubStepStatus<T, P, impl Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>> {
    InitSubStepStatus::Saturating {
        start_time,
        future: WrappedTransposer::init(
            transposer,
            rng_seed,
            start_time,
            shared_step_state,
            0,
            todo!()
        ),
    }
}

pub fn new_init_sub_step<T: Transposer, P: SharedPointerKind, S: InputState<T>>(
    transposer: T,
    rng_seed: [u8; 32],
    start_time: T::Time,
    shared_step_state: NonNull<UnsafeCell<S>>,
) -> impl SubStep<T, P> {
    new_init_sub_step_internal(transposer, rng_seed, start_time, shared_step_state)
}
