mod expire_handle_factory;
mod future_input_container;
mod interpolate_context;
// mod interpolation;
mod sub_step;
mod sub_step_update_context;
mod time;
mod transposer_metadata;
mod wrapped_transposer;

// #[cfg(test)]
// mod test;

use core::task::Waker;
use std::any::TypeId;
use std::ptr::NonNull;
use std::task::Poll;

use archery::{ArcTK, SharedPointer, SharedPointerKind};
use future_input_container::{FutureInputContainer, FutureInputContainerGuard};
use sub_step::init_sub_step::new_init_boxed_sub_step;
use sub_step::scheduled_sub_step::new_scheduled_boxed_sub_step;
use sub_step::{BoxedSubStep, StartSaturateErr};
use wrapped_transposer::WrappedTransposer;

use crate::transposer::Transposer;

use super::input_state_manager::InputStateManager;
use super::output_event_manager::OutputEventManager;
use super::{TransposerInput, TransposerInputEventHandler};

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

pub struct Step<'t, T: Transposer + 't, P: SharedPointerKind + 't = ArcTK> {
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
    uuid_prev: Option<uuid::Uuid>,
}

impl<'a, T: Transposer + 'a, P: SharedPointerKind + 'a> Drop for Step<'a, T, P> {
    fn drop(&mut self) {
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

    pub fn get_input_state(&mut self) -> &mut InputStateManager<T> {
        let mut input_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        &mut unsafe { input_state.as_mut() }.1
    }

    pub fn get_output_state(&mut self) -> &mut OutputEventManager<T> {
        let mut input_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        &mut unsafe { input_state.as_mut() }.0
    }

    pub fn new_init(transposer: T, start_time: T::Time, rng_seed: [u8; 32]) -> Self {
        let shared_step_state = Self::new_shared_step_state();
        let uuid_self = uuid::Uuid::new_v4();
        let uuid_prev = None;
        let init_sub_step =
            new_init_boxed_sub_step(transposer, rng_seed, start_time, shared_step_state);

        Self {
            steps: vec![init_sub_step],
            status: StepStatus::Saturating(0),
            time: start_time,
            shared_step_state,
            event_count: 0,
            can_produce_events: true,
            #[cfg(debug_assertions)]
            uuid_self,
            #[cfg(debug_assertions)]
            uuid_prev,
        }
    }

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

        let next_step = match (next_scheduled_time, next_input) {
            (None, None) => return Ok(None),
            (None, Some(i)) => i.take_sub_step(),
            (Some(t), None) => new_scheduled_boxed_sub_step::<'a, T, P>(t),
            (Some(t), Some(i)) => {
                if i.get_time() > t {
                    new_scheduled_boxed_sub_step::<'a, T, P>(t)
                } else {
                    i.take_sub_step()
                }
            }
        };

        let time = next_step.as_ref().get_time();

        Ok(Some(Self {
            steps: vec![next_step],
            status: StepStatus::Unsaturated,
            time,
            shared_step_state: Self::new_shared_step_state(),
            event_count: 0,
            can_produce_events: true,
            #[cfg(debug_assertions)]
            uuid_self: uuid::Uuid::new_v4(),
            #[cfg(debug_assertions)]
            uuid_prev: Some(self.uuid_self),
        }))
    }

    pub fn next_scheduled_unsaturated(&self) -> Result<Option<Self>, NextUnsaturatedErr>
    where
        T: Clone,
    {
        self.next_unsaturated(&mut None)
    }

    pub fn start_saturate_take(&mut self, prev: &mut Self) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != Some(prev.uuid_self) {
            return Err(SaturateErr::IncorrectPrevious);
        }

        self.start_saturate(prev.take()?)
    }

    pub fn start_saturate_clone(&mut self, prev: &Self) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != Some(prev.uuid_self) {
            return Err(SaturateErr::IncorrectPrevious);
        }

        self.start_saturate(prev.clone()?)
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

    fn start_saturate(
        &mut self,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    ) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        *self.get_output_state() = OutputEventManager::new_with_swallow_count(self.event_count);
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
            })
    }

    pub fn desaturate(&mut self) {
        match self.get_step_status_mut() {
            ActiveStepStatusMut::Saturated(step) => step.as_mut().desaturate(),
            ActiveStepStatusMut::Saturating(step) => step.as_mut().desaturate(),
            _ => {}
        }
        self.status = StepStatus::Unsaturated;
    }

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
                    if let Some(output_event) = self.get_output_state().try_take_value() {
                        self.event_count += 1;
                        break Ok(StepPoll::Emitted(output_event));
                    }

                    if let Some(type_id) = self.get_input_state().get_requested_input_type_id() {
                        break Ok(StepPoll::StateRequested(type_id));
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
                                self.steps
                                    .push(new_scheduled_boxed_sub_step::<'a, T, P>(t_time));
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

    pub fn get_requested_input<I: TransposerInput<Base = T>>(&mut self) -> Option<I>
    where
        T: TransposerInputEventHandler<I>,
    {
        self.get_input_state().get_requested_input()
    }

    pub fn provide_input_state<I: TransposerInput<Base = T>>(
        &mut self,
        input: I,
        input_state: I::InputState,
    ) -> Result<(), I::InputState>
    where
        T: TransposerInputEventHandler<I>,
    {
        let input_state = NonNull::from(Box::leak(Box::new(input_state)));
        self.get_input_state()
            .provide_input_state(input, input_state)
            .map_err(|ptr| *unsafe { Box::from_raw(ptr.as_ptr()) })
    }

    // pub fn interpolate(&self, time: T::Time) -> Result<impl Interpolation<T>, InterpolateErr> {
    //     let wrapped_transposer = match &self.status {
    //         StepStatus::Saturated { wrapped_transposer } => wrapped_transposer.clone(),
    //         _ => return Err(InterpolateErr::NotSaturated),
    //     };

    //     #[cfg(debug_assertions)]
    //     if time < wrapped_transposer.metadata.last_updated.time {
    //         return Err(InterpolateErr::TimePast);
    //     }

    //     Ok(new_interpolation(time, wrapped_transposer))
    // }

    pub fn get_time(&self) -> T::Time {
        self.time
    }

    pub fn is_unsaturated(&self) -> bool {
        matches!(self.status, StepStatus::Unsaturated)
    }

    pub fn is_saturating(&self) -> bool {
        matches!(self.status, StepStatus::Saturating(_))
    }

    pub fn is_saturated(&self) -> bool {
        matches!(self.status, StepStatus::Saturated)
    }

    pub fn can_produce_events(&self) -> bool {
        self.can_produce_events
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StepPoll<T: Transposer> {
    Emitted(T::OutputEvent),
    StateRequested(TypeId),
    Pending,
    Ready,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PollErr {
    Unsaturated,
    Saturated,
}

#[derive(Debug)]
pub enum InterpolateErr {
    NotSaturated,
    #[cfg(debug_assertions)]
    TimePast,
}

#[derive(Debug)]
pub enum NextUnsaturatedErr {
    NotSaturated,
    #[cfg(debug_assertions)]
    InputPastOrPresent,
}

#[derive(Debug)]
pub enum SaturateErr {
    PreviousNotSaturated,
    SelfNotUnsaturated,
    #[cfg(debug_assertions)]
    IncorrectPrevious,
}
