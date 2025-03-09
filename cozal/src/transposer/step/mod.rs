mod expire_handle_factory;
mod future_input_container;
mod interpolate_context;
mod interpolation;
mod pre_init_step;
mod sub_step;
mod sub_step_update_context;
mod time;
mod transposer_metadata;
mod wrapped_transposer;
mod init_step;
mod init_context;
mod previous_step;

#[cfg(test)]
mod test;

use core::task::Waker;
use std::ptr::NonNull;
use std::task::Poll;

use archery::{ArcTK, SharedPointer, SharedPointerKind};
use previous_step::PreviousStep;
use sub_step::scheduled_sub_step::ScheduledSubStep;
use sub_step::{BoxedSubStep, StartSaturateErr};
use wrapped_transposer::WrappedTransposer;

use crate::transposer::Transposer;

use super::input_erasure::{ErasedInput, ErasedInputState};
use super::input_state_manager::InputStateManager;
use super::output_event_manager::OutputEventManager;

pub use future_input_container::{FutureInputContainer, FutureInputContainerGuard};
pub use interpolation::Interpolation;
pub use pre_init_step::PreInitStep;
pub use sub_step::boxed_input::BoxedInput;

#[derive(Debug)]
enum StepStatus {
    // all sub steps are unsaturated.
    Unsaturated,
    // all steps before step[i] are saturated. step[i] is saturating. all steps after step[i] are unsaturated.
    Saturating(usize),
    // all sub steps are saturated. there are no remaining scheduled sub steps at this time.
    Saturated,
}

enum ActiveStepStatusRef<'a, T: Transposer, P: SharedPointerKind> {
    // reference to the first unsaturated sub step.
    Unsaturated,
    // reference to the currently saturating sub step.
    Saturating,
    // reference to the last saturated sub step.
    Saturated(&'a SharedPointer<WrappedTransposer<T, P>, P>),
}

enum ActiveStepStatusMut<'a, 't, T: Transposer + 't, P: SharedPointerKind + 't> {
    // reference to the first unsaturated sub step.
    Unsaturated(&'a mut BoxedSubStep<'t, T, P>),
    // reference to the currently saturating sub step.
    Saturating(&'a mut BoxedSubStep<'t, T, P>),
    // reference to the last saturated sub step.
    Saturated(&'a mut BoxedSubStep<'t, T, P>),
}

/// A step is a structure that allows for the transposer to be thought of as a state machine.
///
/// A step represents the change that occurs to the transposer at a single point in time.
///
/// A step can be in one of three states:
///
/// - Unsaturated: The step is ready to recieve a transposer (and some additional metadata like the scheduled events)
///   and begin processing it.
///
/// - Saturating: The step is in the process of saturating. This means there is some async method on the transposer
///   that has not yet completed. This could be a future that is waiting on some input.
///
/// - Saturated: The step has completed saturating. This means that all async methods on the transposer have completed,
///   and that the transposer is available to either perform interpolation or to be used in the next step.
///
/// A step can move between the states in the following ways:
///
/// - Unsaturated -> Saturating: When the `start_saturate_clone` or `start_saturate_take` methods are called.
///
/// - Saturating -> Saturated: When polling the step returns `Poll::Ready`.
///
/// - (Saturating or Saturated) -> Unsaturated: When the `desaturate` method is called.
///
/// - Saturated -> Unsaturated: When the `start_saturate_take` method is called on the _next_ step.
///
/// Steps are only created by calling `new_init` (at the very beginning to get things started) or by calling
/// `next_unsaturated` or `next_scheduled_unsaturated` on an existing step.
#[derive(Debug)]
pub struct Step<'t, T: Transposer + 't, P: SharedPointerKind + 't = ArcTK> {
    sequence_number: usize,

    steps: Vec<BoxedSubStep<'t, T, P>>,
    status: StepStatus,

    time: T::Time,

    // this is considered the owner of the input state.
    // we are responsible for dropping it.
    shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    event_count: usize,
    can_produce_events: bool,

    #[cfg(debug_assertions)]
    uuid_self: uuid::Uuid,
    #[cfg(debug_assertions)]
    uuid_prev: uuid::Uuid,
}

impl<'a, T: Transposer + 'a, P: SharedPointerKind + 'a> Drop for Step<'a, T, P> {
    fn drop(&mut self) {
        // doesn't matter if there are non-null pointers to this in steps since they won't access this during drop.
        Self::drop_shared_step_state(self.shared_step_state);
    }
}

impl<'a, T: Transposer + 'a, P: SharedPointerKind + 'a> Step<'a, T, P> {
    fn drop_shared_step_state(ptr: NonNull<(OutputEventManager<T>, InputStateManager<T>)>) {
        unsafe { NonNull::drop_in_place(ptr) }
    }

    fn new_shared_step_state() -> NonNull<(OutputEventManager<T>, InputStateManager<T>)> {
        let input_state = Box::default();
        NonNull::from(Box::leak(input_state))
    }

    fn get_step_status_ref(&self) -> ActiveStepStatusRef<'_, T, P> {
        match self.status {
            StepStatus::Unsaturated => ActiveStepStatusRef::Unsaturated,
            StepStatus::Saturating(_) => ActiveStepStatusRef::Saturating,
            StepStatus::Saturated => {
                let step = self.steps.last().unwrap();
                let transposer = step.as_ref().get_finished_transposer().unwrap();
                ActiveStepStatusRef::Saturated(transposer)
            }
        }
    }

    fn get_step_status_mut(&mut self) -> ActiveStepStatusMut<'_, 'a, T, P> {
        match self.status {
            StepStatus::Unsaturated => {
                let step = self.steps.first_mut().unwrap();
                ActiveStepStatusMut::Unsaturated(step)
            }
            StepStatus::Saturating(i) => {
                let step = self.steps.get_mut(i).unwrap();
                ActiveStepStatusMut::Saturating(step)
            }
            StepStatus::Saturated => {
                let step = self.steps.last_mut().unwrap();
                ActiveStepStatusMut::Saturated(step)
            }
        }
    }

    fn get_input_state_mut(&mut self) -> &mut InputStateManager<T> {
        &mut unsafe { self.shared_step_state.as_mut() }.1
    }

    fn get_output_state_mut(&mut self) -> &mut OutputEventManager<T> {
        let mut input_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        &mut unsafe { input_state.as_mut() }.0
    }

    /// Create a new step that is ready to be saturated.
    ///
    /// This will compare the time of the next scheduled event in the current schedule with the time
    /// of `next_inputs`, and either take the next input event from the container to produce a step, or
    /// leave it in the container and produce a step that will handle the scheduled event.
    pub fn next_unsaturated<F: FutureInputContainer<'a, T, P>>(
        &self,
        next_inputs: &mut F,
    ) -> Result<Option<Self>, NextUnsaturatedErr>
    where
        T: Clone,
    {
        let wrapped_transposer = match self.get_step_status_ref() {
            ActiveStepStatusRef::Saturated(t) => t,
            _ => return Err(NextUnsaturatedErr::NotSaturated),
        };

        let next_scheduled_time = wrapped_transposer
            .metadata
            .get_next_scheduled_time()
            .map(|t| t.time);

        let next_input = next_inputs.next();

        if let Some(i) = next_input.as_ref() {
            if i.get_time() <= self.time {
                return Err(NextUnsaturatedErr::InputPastOrPresent);
            }
        }

        let (time, next_scheduled_time, next_input) = match (next_scheduled_time, next_input) {
            (None, None) => return Ok(None),
            (None, Some(i)) => (i.get_time(), None, Some(i)),
            (Some(t), None) => (t, Some(t), None),
            (Some(t), Some(i)) => {
                let i_time = i.get_time();
                if i_time > t {
                    (t, Some(t), None)
                } else {
                    (i_time, None, Some(i))
                }
            }
        };

        let steps = match (next_scheduled_time, next_input) {
            (None, Some(i)) => {
                let mut steps = Vec::new();
                let mut front = Some(i);
                loop {
                    let (item, new_front) = match front.take() {
                        Some(front) => {
                            if front.get_time() != time {
                                break;
                            }
                            front.take_sub_step()
                        }
                        None => break,
                    };
                    front = new_front;
                    steps.push(item.into());
                }
                steps
            }
            (Some(t), None) => vec![ScheduledSubStep::new_boxed(t)],
            _ => unreachable!(),
        };

        Ok(Some(Self {
            sequence_number: self.sequence_number + 1,
            steps,
            status: StepStatus::Unsaturated,
            time,
            shared_step_state: Self::new_shared_step_state(),
            event_count: 0,
            can_produce_events: true,
            #[cfg(debug_assertions)]
            uuid_self: uuid::Uuid::new_v4(),
            #[cfg(debug_assertions)]
            uuid_prev: self.uuid_self,
        }))
    }

    /// Create a new step that is ready to be saturated.
    ///
    /// This will only create a step from a scheduled event, and should be used if you know there
    /// isn't another input event in the future.
    pub fn next_scheduled_unsaturated(&self) -> Result<Option<Self>, NextUnsaturatedErr>
    where
        T: Clone,
    {
        self.next_unsaturated(&mut None)
    }

    /// Begin saturating the step by taking the transposer and metadata from the previous step.
    ///
    /// This moves the previous step from Saturated to Unsaturated, and the current step from Unsaturated to Saturating.
    ///
    /// # Errors
    ///
    /// - If the previous step is not Saturated.
    /// - If the current step is not Unsaturated.
    /// - If the previous step's UUID does not match the current step's UUID. (only when debug assertions are enabled)
    pub fn start_saturate_take(&mut self, prev: &mut impl PreviousStep<T, P>) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != prev.get_uuid() {
            return Err(SaturateErr::IncorrectPrevious);
        }

        self.start_saturate(prev.take()?)
    }

    /// Begin saturating the step by cloning the transposer and metadata from the previous step.
    ///
    /// This moves the current step from Unsaturated to Saturating, without changing the previous step.
    ///
    /// # Errors
    ///
    /// - If the previous step is not Saturated.
    /// - If the current step is not Unsaturated.
    /// - If the previous step's UUID does not match the current step's UUID. (only when debug assertions are enabled)
    pub fn start_saturate_clone(&mut self, prev: & impl PreviousStep<T, P>) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != prev.get_uuid() {
            return Err(SaturateErr::IncorrectPrevious);
        }

        self.start_saturate(prev.clone()?)
    }

    fn start_saturate(
        &mut self,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    ) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        *self.get_output_state_mut() = OutputEventManager::new_with_swallow_count(self.event_count);
        let shared_step_state = self.shared_step_state;
        let first = match self.get_step_status_mut() {
            ActiveStepStatusMut::Unsaturated(first) => first,
            _ => return Err(SaturateErr::SelfNotUnsaturated),
        };

        first
            .as_mut()
            .start_saturate(SharedPointer::clone(&wrapped_transposer), shared_step_state)
            .map_err(|e| match e {
                StartSaturateErr::SubStepTimeIsPast => panic!(),
                StartSaturateErr::NotUnsaturated => SaturateErr::SelfNotUnsaturated,
            })?;

        self.status = StepStatus::Saturating(0);
        Ok(())
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
    pub fn desaturate(&mut self) {
        match self.get_step_status_mut() {
            ActiveStepStatusMut::Saturated(step) => step.as_mut().desaturate(),
            ActiveStepStatusMut::Saturating(step) => step.as_mut().desaturate(),
            _ => {}
        }

        Self::drop_shared_step_state(self.shared_step_state);
        self.shared_step_state = Self::new_shared_step_state();
        self.status = StepStatus::Unsaturated;
    }

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
    pub fn poll(&mut self, waker: &Waker) -> Result<StepPoll<T>, PollErr>
    where
        T: Clone,
    {
        loop {
            let time = self.get_time();
            let step_count = self.steps.len();
            let current_index = match self.status {
                StepStatus::Saturating(i) => i,
                _ => return Err(PollErr::Unsaturated),
            };
            let mut sub_step = match self.get_step_status_mut() {
                ActiveStepStatusMut::Saturating(step) => step.as_mut(),
                _ => unreachable!(),
            };

            match sub_step.as_mut().poll(waker)? {
                Poll::Pending => {
                    if let Some(output_event) = self.get_output_state_mut().try_take_value() {
                        self.event_count += 1;
                        break Ok(StepPoll::Emitted(output_event));
                    }

                    if let Some(erased_input) = self.get_input_state_mut().try_accept_request() {
                        break Ok(StepPoll::StateRequested(erased_input));
                    }

                    break Ok(StepPoll::Pending);
                }
                Poll::Ready(()) => {
                    // if we just finished saturating the last step
                    if current_index + 1 == step_count {
                        // check if there are any scheduled events at this time, and if so push a step to handle them.
                        if let Some(t) = sub_step
                            .get_finished_transposer()
                            .unwrap()
                            .metadata
                            .get_next_scheduled_time()
                        {
                            let t_time = t.time;
                            if t_time == time {
                                self.steps.push(ScheduledSubStep::new_boxed(t_time));
                                self.status = StepStatus::Saturating(current_index + 1);
                                continue;
                            }
                        }

                        // if there are no scheduled events, we are done.
                        self.status = StepStatus::Saturated;
                        self.can_produce_events = false;
                        break Ok(StepPoll::Ready);
                    } else {
                        // advance to the next step.
                        let wrapped_transposer = sub_step.take_finished_transposer().unwrap();
                        let shared_step_state = self.shared_step_state;
                        let next_sub_step = self.steps.get_mut(current_index + 1).unwrap();
                        next_sub_step
                            .as_mut()
                            .start_saturate(wrapped_transposer, shared_step_state)
                            .unwrap();
                        self.status = StepStatus::Saturating(current_index + 1);
                        continue;
                    }
                }
            }
        }
    }

    /// Provide the input state that was requested by the step during polling.
    ///
    /// This will return `Ok(())` if the input state was successfully provided, and `Err(input_state)` if the
    /// input state was not requested, or if the input state was not of the correct type.
    pub fn provide_input_state(
        &mut self,
        erased_state: Box<ErasedInputState<T>>,
    ) -> Result<(), Box<ErasedInputState<T>>> {
        self.get_input_state_mut().provide_input_state(erased_state)
    }

    /// Begin interpolating the outut state of the step to the given time.
    ///
    /// This will return an `Interpolation` object that can be used like a future. While this is a future,
    /// it must be polled manually since input state may need to be provided between polls.
    pub fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, P>, InterpolateErr>
    where
        T: Clone,
    {
        let wrapped_transposer = match self.get_step_status_ref() {
            ActiveStepStatusRef::Saturated(wrapped_transposer) => wrapped_transposer.clone(),
            _ => return Err(InterpolateErr::NotSaturated),
        };

        #[cfg(debug_assertions)]
        if let Some(t) = wrapped_transposer.metadata.last_updated {
            if t.time > time {
                return Err(InterpolateErr::TimePast);
            }
        }

        Ok(Interpolation::new(time, wrapped_transposer))
    }

    /// Discard the step, extracting and returning all input events, so they can be reused, perhaps with
    /// new events added, or some of the events removed.
    ///
    /// They will be emitted in sorted order (the order the transposer would see them).
    pub fn drain_inputs(mut self) -> impl IntoIterator<Item = BoxedInput<'a, T, P>> {
        // need to desaturate before dropping self, since saturating steps may point to shared state.
        for step in &mut self.steps {
            step.as_mut().desaturate();
        }

        let steps = core::mem::take(&mut self.steps);

        steps.into_iter().filter_map(|step| step.try_into().ok())
    }

    /// Get the time of the step.
    pub fn get_time(&self) -> T::Time {
        self.time
    }

    /// true if the step is unsaturated.
    pub fn is_unsaturated(&self) -> bool {
        matches!(self.status, StepStatus::Unsaturated)
    }

    /// true if the step is saturating.
    pub fn is_saturating(&self) -> bool {
        matches!(self.status, StepStatus::Saturating(_))
    }

    /// true if the step is saturated.
    pub fn is_saturated(&self) -> bool {
        matches!(self.status, StepStatus::Saturated)
    }

    /// true if the step might still produce events.
    ///
    /// generally this will only be false if the step has ever been fully saturated.
    pub fn can_produce_events(&self) -> bool {
        self.can_produce_events
    }
}

impl<'a, T: Transposer + 'a, P: SharedPointerKind + 'a> PreviousStep<T, P> for Step<'a, T, P> {

    #[cfg(debug_assertions)]
    fn get_uuid(&self) -> uuid::Uuid {
        self.uuid_self
    }

    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr> {
        match self.get_step_status_mut() {
            ActiveStepStatusMut::Saturated(step) => {
                Ok(step.as_mut().take_finished_transposer().unwrap())
            }
            _ => Err(SaturateErr::PreviousNotSaturated),
        }
    }

    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr> {
        match self.get_step_status_ref() {
            ActiveStepStatusRef::Saturated(t) => Ok(SharedPointer::clone(t)),
            _ => Err(SaturateErr::PreviousNotSaturated),
        }
    }
}

impl<T: Transposer, P: SharedPointerKind> Ord for Step<'_, T, P> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.sequence_number.cmp(&other.sequence_number)
    }
}

impl<T: Transposer, P: SharedPointerKind> PartialOrd for Step<'_, T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Transposer, P: SharedPointerKind> PartialEq for Step<'_, T, P> {
    fn eq(&self, other: &Self) -> bool {
        self.sequence_number == other.sequence_number
    }
}

impl<T: Transposer, P: SharedPointerKind> Eq for Step<'_, T, P> {}

/// The result of polling a step.
#[derive(PartialEq, Eq)]
pub enum StepPoll<T: Transposer> {
    /// The step has emitted an event. The waker may never be called, and the caller is responsible for
    /// calling `poll` again after handling the event.
    Emitted(T::OutputEvent),

    /// The step has requested an input state. The waker may never be called, and the caller is responsible for
    /// calling `poll` again after providing the requested input state.
    ///
    /// the type id is the type id of the input that was requested.
    ///
    /// the specific input can be retrieved by calling `get_requested_input` on the step, then provided by calling
    /// `provide_input_state` on the step.
    StateRequested(Box<ErasedInput<T>>),

    /// The step is still pending. The waker will be called when the step is ready to be polled again.
    Pending,

    /// The step is now saturated.
    Ready,
}

impl<T: Transposer> std::fmt::Debug for StepPoll<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepPoll::Emitted(_) => write!(f, "StepPoll::Emitted"),
            StepPoll::StateRequested(_) => write!(f, "StepPoll::StateRequested"),
            StepPoll::Pending => write!(f, "StepPoll::Pending"),
            StepPoll::Ready => write!(f, "StepPoll::Ready"),
        }
    }
}

/// The error result of polling a step.
#[derive(Debug, PartialEq, Eq)]
pub enum PollErr {
    /// The step is unsaturated.
    Unsaturated,

    /// The step is saturated.
    Saturated,
}

/// The error result of interpolating a step.
#[derive(Debug)]
pub enum InterpolateErr {
    /// The step is not saturated.
    NotSaturated,

    /// The time to interpolate to is in the past.
    ///
    /// This is only available when debug assertions are enabled.
    #[cfg(debug_assertions)]
    TimePast,
}

/// The error result of getting the next unsaturated step.
#[derive(Debug)]
pub enum NextUnsaturatedErr {
    /// The step is not saturated.
    NotSaturated,

    /// The input event is in the past or present.
    ///
    /// This is only available when debug assertions are enabled.
    #[cfg(debug_assertions)]
    InputPastOrPresent,
}

/// The error result of starting to saturate a step.
#[derive(Debug)]
pub enum SaturateErr {
    /// The previous step is not saturated.
    PreviousNotSaturated,

    /// The current step is not unsaturated.
    SelfNotUnsaturated,

    /// The previous step's UUID does not match the current step's UUID.
    ///
    /// This is only available when debug assertions are enabled.
    #[cfg(debug_assertions)]
    IncorrectPrevious,
}
