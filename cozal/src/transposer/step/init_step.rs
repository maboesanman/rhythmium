use std::{
    pin::Pin,
    task::{Poll, Waker},
};

use std::fmt::Debug;

use archery::{ArcTK, SharedPointer, SharedPointerKind};

use crate::transposer::Transposer;

use super::{
    step::{InterpolateErr, NextUnsaturatedErr, PollErr, SaturateErr}, sub_step::{init_sub_step::InitSubStep, scheduled_sub_step::ScheduledSubStep}, wrapped_transposer::WrappedTransposer, BoxedInput, FutureInputContainer, Interpolation, PossiblyInitStep, PreInitStep, Step, StepPoll
};

pub struct InitStep<T: Transposer, P: SharedPointerKind = ArcTK> {
    sub_step: Pin<Box<InitSubStep<T, P>>>,

    #[cfg(debug_assertions)]
    uuid_self: uuid::Uuid,
}

impl<T: Transposer, P: SharedPointerKind> Debug for InitStep<T, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitStep").field("sub_step", &self.sub_step).field("uuid_self", &self.uuid_self).finish()
    }
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
}

impl<'a, T: Transposer + Clone + 'a, P: SharedPointerKind + 'a> PossiblyInitStep<'a, T, P>
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
    
    fn next_unsaturated(
        &self,
        next_inputs: &mut dyn FutureInputContainer<'a, T, P>,
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

        let next_input = next_inputs.peek_time();

        let (time, next_scheduled_time, next_input) = match (next_scheduled_time, next_input) {
            (None, None) => return Ok(None),
            (None, Some(i)) => (i, None, Some(i)),
            (Some(t), None) => (t, Some(t), None),
            (Some(t), Some(i)) => {
                let i_time = i;
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
                while next_inputs.peek_time() == Some(i) {
                    steps.push(next_inputs.take_next().unwrap().into());
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
    
    fn desaturate(&mut self) {
        todo!()
    }
    
    fn poll(&mut self, waker: &Waker) -> Result<super::StepPoll<T>, PollErr>
    where
        T: Clone {
        Ok(match self.sub_step.as_mut().poll(waker)?{
            Poll::Ready(()) => StepPoll::Ready,
            Poll::Pending => StepPoll::Pending,
        })
    }
    
    fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, P>, InterpolateErr>
    where
        T: Clone,
    {
        let wrapped_transposer = match &*self.sub_step {
            InitSubStep::Saturated { wrapped_transposer } => wrapped_transposer.clone(),
            _ => return Err(InterpolateErr::NotSaturated),
        };

        Ok(Interpolation::new(time, wrapped_transposer))
    }
    
    fn is_unsaturated(&self) -> bool {
        false
    }
    
    fn is_saturating(&self) -> bool {
        matches!(*self.sub_step, InitSubStep::Saturating { .. })
    }
    
    fn is_saturated(&self) -> bool {
        matches!(*self.sub_step, InitSubStep::Saturated { .. })
    }

    fn get_time(&self) -> <T as Transposer>::Time {
        unimplemented!()
    }
}
