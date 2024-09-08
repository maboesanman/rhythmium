use std::{any::{Any, TypeId}, cell::UnsafeCell, future::Future, marker::PhantomData, ptr::NonNull};

use archery::{ArcTK, SharedPointer, SharedPointerKind};

use crate::transposer::Transposer;

use super::{wrapped_transposer::WrappedTransposer, InputState};

pub mod init_sub_step;
pub mod input_sub_step;
pub mod scheduled_sub_step;

pub trait SubStep<T: Transposer, P: SharedPointerKind> {
    fn is_input(&self) -> bool { false }
    fn input_sort(&self) -> Option<(u64, TypeId)> { None }
    fn is_init(&self) -> bool { false }
    fn is_scheduled(&self) -> bool { false }
    fn is_unsaturated(&self) -> bool;
    fn is_saturating(&self) -> bool;
    fn is_saturated(&self) -> bool;
    fn get_time(&self) -> T::Time;

    fn cmp(&self, other: &dyn SubStep<T, P>) -> std::cmp::Ordering;
}

// struct InitSubStep<T: Transposer, Is: InputState<T>, P: SharedPointerKind> { }



