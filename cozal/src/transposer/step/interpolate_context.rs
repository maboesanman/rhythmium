use std::ptr::NonNull;

use archery::SharedPointerKind;

// use super::lazy_state::LazyState;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::{
    CurrentTimeContext, InputStateManagerContext, InterpolateContext, InterpolateContextInner, LastUpdatedTimeContext
};
use crate::transposer::input_state_manager::InputStateManager;
use crate::transposer::Transposer;

pub struct StepInterpolateContext<'update, T: Transposer, P: SharedPointerKind> {
    interpolation_time: T::Time,
    metadata: &'update TransposerMetaData<T, P>,
    // the ownership of this is effectively shared up a couple levels in the step, so we can't
    // store live references to it.
    shared_step_state: NonNull<InputStateManager<T>>,
}

impl<'update, T: Transposer, P: SharedPointerKind> StepInterpolateContext<'update, T, P> {
    pub fn new(
        interpolation_time: T::Time,
        metadata: &'update TransposerMetaData<T, P>,
        shared_step_state: NonNull<InputStateManager<T>>,
    ) -> Self {
        Self {
            interpolation_time,
            metadata,
            shared_step_state,
        }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> InterpolateContextInner<'update, T>
    for StepInterpolateContext<'update, T, P>
{
}

impl<'update, T: Transposer, P: SharedPointerKind> InputStateManagerContext<'update, T>
    for StepInterpolateContext<'update, T, P>
{
    fn get_input_state_manager(&mut self) -> NonNull<InputStateManager<T>> {
        NonNull::from(unsafe { self.shared_step_state.as_mut() })
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> CurrentTimeContext<T>
    for StepInterpolateContext<'update, T, P>
{
    fn current_time(&self) -> <T as Transposer>::Time {
        self.interpolation_time
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> LastUpdatedTimeContext<T>
    for StepInterpolateContext<'update, T, P>
{
    fn last_updated_time(&self) -> <T as Transposer>::Time {
        self.metadata.last_updated.time
    }
}
