use std::cell::UnsafeCell;
use std::ptr::NonNull;

use archery::SharedPointerKind;

// use super::lazy_state::LazyState;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::{
    CurrentTimeContext, InputStateManagerContext, InterpolateContext, LastUpdatedTimeContext,
};
use crate::transposer::input_state_manager::InputStateManager;
use crate::transposer::Transposer;

pub struct StepInterpolateContext<'update, T: Transposer, P: SharedPointerKind, S> {
    interpolation_time: T::Time,
    metadata: &'update TransposerMetaData<T, P>,
    // the ownership of this is effectively shared up a couple levels in the step, so we can't
    // store live references to it.
    input_state: NonNull<(S, InputStateManager<T>)>,
}

impl<'update, T: Transposer, P: SharedPointerKind, S> StepInterpolateContext<'update, T, P, S> {
    pub fn new(
        interpolation_time: T::Time,
        metadata: &'update TransposerMetaData<T, P>,
        input_state: NonNull<(S, InputStateManager<T>)>,
    ) -> Self {
        Self {
            interpolation_time,
            metadata,
            input_state,
        }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, S> InterpolateContext<'update, T>
    for StepInterpolateContext<'update, T, P, S>
{
}

impl<'update, T: Transposer, P: SharedPointerKind, S> InputStateManagerContext<'update, T>
    for StepInterpolateContext<'update, T, P, S>
{
    fn get_input_state_manager(&mut self) -> NonNull<InputStateManager<T>> {
        NonNull::from(unsafe { &self.input_state.as_ref().1 })
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, S> CurrentTimeContext<T>
    for StepInterpolateContext<'update, T, P, S>
{
    fn current_time(&self) -> <T as Transposer>::Time {
        self.interpolation_time
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, S> LastUpdatedTimeContext<T>
    for StepInterpolateContext<'update, T, P, S>
{
    fn last_updated_time(&self) -> <T as Transposer>::Time {
        self.metadata.last_updated.time
    }
}
