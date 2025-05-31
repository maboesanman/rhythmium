mod expire_handle_factory;
mod future_input_container;
mod init_context;
mod init_step;
mod interpolate_context;
mod interpolation;
mod pre_init_step;
mod step;
mod sub_step;
mod sub_step_update_context;
mod time;
mod transposer_metadata;
mod wrapped_transposer;

#[cfg(test)]
mod test;

use std::task::Waker;

use archery::{SharedPointer, SharedPointerKind};
pub use future_input_container::FutureInputContainer;
pub use init_step::InitStep;
pub use interpolation::Interpolation;
pub use pre_init_step::PreInitStep;
pub use step::{InterpolateErr, NextUnsaturatedErr, PollErr, SaturateErr, Step, StepPoll};
pub use sub_step::boxed_input::BoxedInput;
use wrapped_transposer::WrappedTransposer;

use super::Transposer;

/// A trait for shared functionality between the `InitStep` and `Step` types.
pub trait PossiblyInitStep<'a, T: Transposer + Clone + 'a, P: SharedPointerKind + 'a> {
    /// Create a new step that is ready to be saturated.
    ///
    /// This will compare the time of the next scheduled event in the current schedule with the time
    /// of `next_inputs`, and either take the next input event from the container to produce a step, or
    /// leave it in the container and produce a step that will handle the scheduled event.
    fn next_unsaturated(
        &self,
        next_inputs: &mut dyn FutureInputContainer<'a, T, P>,
    ) -> Result<Option<Step<'a, T, P>>, NextUnsaturatedErr>;

    /// Create a new step that is ready to be saturated.
    ///
    /// This will only create a step from a scheduled event, and should be used if you know there
    /// isn't another input event in the future.
    fn next_scheduled_unsaturated(&self) -> Result<Option<Step<'a, T, P>>, NextUnsaturatedErr> {
        self.next_unsaturated(&mut None)
    }

    /// Desaturate the step.
    ///
    /// This will move the step from Saturated or Saturating to Unsaturated, and all sub steps will be desaturated.
    ///
    /// When you desaturate a step, subsequent saturations *WILL NOT* result in re-emissions of the events that
    /// were emitted during the previous saturation, even if the previous saturation never completed. Think of this
    /// as the step remembering the events that were emitted, and skipping them if they are emmitted when the step
    /// is re-saturated. This will not prevent identical events from being emitted, however. The only observable
    /// difference (from the perspective of the transposer) is that the `emit_event` futures will immediately return
    /// `Poll::Ready` if the event was emitted during the previous saturation.
    ///
    /// This also resets the stored input state
    fn desaturate(&mut self);

    /// Poll a saturated step toward completion.
    ///
    /// While this resembles a future, it is not a future, and has more types of results.
    ///
    /// # Returns
    ///
    /// - If the step is ready, this will move the step from Saturating to Saturated, and return `Ok(StepPoll::Ready)`.
    /// - If the step is not ready:
    ///     - If the step has emitted an event, and is waiting for the event to be extracted, this will return `Ok(StepPoll::Emitted(event))`.
    ///     - If the step has requested an input state and is waiting for it to be provided, this will return `Ok(StepPoll::StateRequested(type_id))`.
    ///
    /// # Errors
    ///
    /// - If the step is unsaturated, this will return `Err(PollErr::Unsaturated)`.
    /// - If the step is saturated, this will return `Err(PollErr::Saturated)`.
    fn poll(&mut self, waker: &Waker) -> Result<StepPoll<T>, PollErr>;

    /// Begin interpolating the outut state of the step to the given time.
    ///
    /// This will return an `Interpolation` object that can be used like a future. While this is a future,
    /// it must be polled manually since input state may need to be provided between polls.
    fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, P>, InterpolateErr>;

    /// get the step's uuid, for debug checks of proper hydration
    #[cfg(debug_assertions)]
    fn get_uuid(&self) -> uuid::Uuid;

    /// remove the wrapped transposer and return it.
    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr>;

    /// clone the wrapped transposer and return it.
    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr>;

    /// true if the step is unsaturated.
    fn is_unsaturated(&self) -> bool;

    /// true if the step is saturating.
    fn is_saturating(&self) -> bool;

    /// true if the step is saturated.
    fn is_saturated(&self) -> bool;

    /// get the time of the step.
    fn get_time(&self) -> T::Time;
}
