use archery::SharedPointerKind;

// use super::lazy_state::LazyState;
use super::transposer_metadata::TransposerMetaData;
use crate::transposer::context::{
    CurrentTimeContext, InputStateContext, InterpolateContext, LastUpdatedTimeContext,
};
use crate::transposer::Transposer;

pub struct StepInterpolateContext<'update, T: Transposer, P: SharedPointerKind> {
    interpolation_time: T::Time,
    metadata: &'update TransposerMetaData<T, P>,
    input_state: &'update T::InputStateManager,
}

impl<'update, T: Transposer, P: SharedPointerKind> StepInterpolateContext<'update, T, P> {
    pub fn new(
        interpolation_time: T::Time,
        metadata: &'update TransposerMetaData<T, P>,
        input_state: &'update T::InputStateManager,
    ) -> Self {
        Self {
            interpolation_time,
            metadata,
            input_state,
        }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind> InterpolateContext<'update, T>
    for StepInterpolateContext<'update, T, P>
{
}

impl<'update, T: Transposer, P: SharedPointerKind> InputStateContext<'update, T>
    for StepInterpolateContext<'update, T, P>
{
    fn get_input_state_manager(&mut self) -> &'update T::InputStateManager {
        self.input_state
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
