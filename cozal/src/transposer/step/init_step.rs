use std::{
    pin::Pin,
    task::{Poll, Waker},
};

use archery::{ArcTK, SharedPointer, SharedPointerKind};

use crate::transposer::Transposer;

use super::{
    FutureInputContainer, FutureInputContainerGuard, Interpolation, PreInitStep, Step,
    previous_step::PreviousStep,
    step::{InterpolateErr, NextUnsaturatedErr, PollErr, SaturateErr},
    sub_step::{init_sub_step::InitSubStep, scheduled_sub_step::ScheduledSubStep},
    wrapped_transposer::WrappedTransposer,
};

pub struct InitStep<T: Transposer, P: SharedPointerKind = ArcTK> {
    sub_step: Pin<Box<InitSubStep<T, P>>>,

    #[cfg(debug_assertions)]
    uuid_self: uuid::Uuid,
}

#[allow(dead_code)]
impl<T: Transposer + Clone, P: SharedPointerKind> InitStep<T, P> {
    /// Create new beginning step.
    ///
    /// This is the first step the transposer undergoes, whic his why it recieves the transposer as an argument, as
    /// opposed to the other steps which get it from the previous step.
    pub fn new(transposer: T, pre_init_step: PreInitStep<T>, rng_seed: [u8; 32]) -> Result<Self, T>
    where
        T: Clone,
    {
        let uuid_self = uuid::Uuid::new_v4();

        let transposer = pre_init_step.execute(transposer)?;
        let init_sub_step = InitSubStep::new(transposer, rng_seed);

        Ok(Self {
            sub_step: Box::pin(init_sub_step),
            #[cfg(debug_assertions)]
            uuid_self,
        })
    }

    /// Create a new step that is ready to be saturated.
    ///
    /// This will compare the time of the next scheduled event in the current schedule with the time
    /// of `next_inputs`, and either take the next input event from the container to produce a step, or
    /// leave it in the container and produce a step that will handle the scheduled event.
    pub fn next_unsaturated<'a, F: FutureInputContainer<'a, T, P>>(
        &self,
        next_inputs: &mut F,
    ) -> Result<Option<Step<'a, T, P>>, NextUnsaturatedErr>
    where
        T: 'a + Clone,
    {
        let wrapped_transposer = match &*self.sub_step {
            InitSubStep::Saturated { wrapped_transposer } => &**wrapped_transposer,
            _ => return Err(NextUnsaturatedErr::NotSaturated),
        };

        let next_scheduled_time = wrapped_transposer
            .metadata
            .get_next_scheduled_time()
            .map(|t| t.time);

        let next_input = next_inputs.next();

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

        #[cfg(debug_assertions)]
        return Ok(Some(Step::new(time, self.uuid_self, steps)));

        #[cfg(not(debug_assertions))]
        return Ok(Some(Step::new(time, steps)));
    }

    /// Create a new step that is ready to be saturated.
    ///
    /// This will only create a step from a scheduled event, and should be used if you know there
    /// isn't another input event in the future.
    pub fn next_scheduled_unsaturated<'a>(
        &self,
    ) -> Result<Option<Step<'a, T, P>>, NextUnsaturatedErr>
    where
        T: Clone,
    {
        self.next_unsaturated(&mut None)
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
    pub fn poll(&mut self, waker: &Waker) -> Result<Poll<()>, PollErr>
    where
        T: Clone,
    {
        self.sub_step.as_mut().poll(waker)
    }

    /// Begin interpolating the output state of the step to the given time.
    ///
    /// This will return an `Interpolation` object that can be used like a future. While this is a future,
    /// it must be polled manually since input state may need to be provided between polls.
    pub fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, P>, InterpolateErr>
    where
        T: Clone,
    {
        let wrapped_transposer = match &*self.sub_step {
            InitSubStep::Saturated { wrapped_transposer } => wrapped_transposer.clone(),
            _ => return Err(InterpolateErr::NotSaturated),
        };

        Ok(Interpolation::new(time, wrapped_transposer))
    }

    /// true if the step is saturating.
    pub fn is_saturating(&self) -> bool {
        matches!(*self.sub_step, InitSubStep::Saturating { .. })
    }

    /// true if the step is saturated.
    pub fn is_saturated(&self) -> bool {
        matches!(*self.sub_step, InitSubStep::Saturated { .. })
    }
}

impl<'a, T: Transposer + Clone + 'a, P: SharedPointerKind + 'a> PreviousStep<T, P>
    for InitStep<T, P>
{
    #[cfg(debug_assertions)]
    fn get_uuid(&self) -> uuid::Uuid {
        self.uuid_self
    }

    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr> {
        self.clone()
    }

    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr> {
        match &*self.sub_step {
            InitSubStep::Saturating { .. } => Err(SaturateErr::PreviousNotSaturated),
            InitSubStep::Saturated { wrapped_transposer } => Ok(wrapped_transposer.clone()),
        }
    }
}
