use std::cell::UnsafeCell;
use std::ptr::NonNull;

use archery::SharedPointerKind;

// use super::lazy_state::LazyState;
use super::transposer_metadata::TransposerMetaData;
use super::InputState;
use crate::transposer::context::{
    CurrentTimeContext, InputStateManagerContext, InterpolateContext, LastUpdatedTimeContext,
};
use crate::transposer::Transposer;

pub struct StepInterpolateContext<'update, T: Transposer, P: SharedPointerKind, Is> {
    interpolation_time: T::Time,
    metadata: &'update TransposerMetaData<T, P>,
    // the ownership of this is effectively shared up a couple levels in the step, so we can't
    // store live references to it.
    input_state: NonNull<UnsafeCell<Is>>,
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> StepInterpolateContext<'update, T, P, Is> {
    pub fn new(
        interpolation_time: T::Time,
        metadata: &'update TransposerMetaData<T, P>,
        input_state: NonNull<UnsafeCell<Is>>,
    ) -> Self {
        Self {
            interpolation_time,
            metadata,
            input_state,
        }
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> InterpolateContext<'update, T>
    for StepInterpolateContext<'update, T, P, Is>
{
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> InputStateManagerContext<'update, T>
    for StepInterpolateContext<'update, T, P, Is>
{
    fn get_input_state_manager(&mut self) -> &mut T::InputStateManager<'update> {
        let input_state: NonNull<UnsafeCell<Is>> = self.input_state;
        let input_state: &UnsafeCell<Is> = unsafe { input_state.as_ref() };
        let input_state: *mut Is = input_state.get();
        let input_state: &mut Is = unsafe { input_state.as_mut() }.unwrap();

        let input_state_manager: &mut T::InputStateManager<'static> = input_state.get_provider();
        let input_state_manager: &mut T::InputStateManager<'update> = unsafe { core::mem::transmute(input_state_manager) };

        input_state_manager
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> CurrentTimeContext<T>
    for StepInterpolateContext<'update, T, P, Is>
{
    fn current_time(&self) -> <T as Transposer>::Time {
        self.interpolation_time
    }
}

impl<'update, T: Transposer, P: SharedPointerKind, Is: InputState<T>> LastUpdatedTimeContext<T>
    for StepInterpolateContext<'update, T, P, Is>
{
    fn last_updated_time(&self) -> <T as Transposer>::Time {
        self.metadata.last_updated.time
    }
}
