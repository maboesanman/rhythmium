use std::future::Future;

use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::{step::{time::ScheduledTime, wrapped_transposer::WrappedTransposer}, Transposer};

use super::SubStep;



enum ScheduledSubStepStatus<T: Transposer, P: SharedPointerKind, Fut> {
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

impl<T, P, Fut> SubStep<T, P> for ScheduledSubStepStatus<T, P, Fut>
where 
    T: Transposer,
    P: SharedPointerKind,
    Fut: Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>,
{
    fn is_scheduled(&self) -> bool {
        true
    }
    fn is_unsaturated(&self) -> bool {
        matches!(self, ScheduledSubStepStatus::Unsaturated { .. })
    }

    fn is_saturating(&self) -> bool {
        matches!(self, ScheduledSubStepStatus::Saturating { .. })
    }

    fn is_saturated(&self) -> bool {
        matches!(self, ScheduledSubStepStatus::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        match self {
            ScheduledSubStepStatus::Unsaturated { time } => *time,
            ScheduledSubStepStatus::Saturating { time, .. } => *time,
            ScheduledSubStepStatus::Saturated { wrapped_transposer } => wrapped_transposer.metadata.last_updated.time,
        }
    }

    fn cmp(&self, other: &dyn SubStep<T, P>) -> std::cmp::Ordering {
        match other.is_scheduled() {
            true => std::cmp::Ordering::Equal,
            false => std::cmp::Ordering::Greater,
        }
    }
}