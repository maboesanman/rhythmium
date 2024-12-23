use std::collections::{BTreeSet, VecDeque};
use std::ops::{Range, RangeInclusive};
use std::ptr::NonNull;
use std::slice::SliceIndex;
use std::task::Poll;

use archery::ArcTK;

use crate::transposer::step::{BoxedInput, Interpolation, PreInitStep, Step};
use crate::transposer::Transposer;


// a collection of Rc which are guranteed not to be cloned outside the collection is Send
// whenever the same collection, but with Arc would be Send, so we do an unsafe impl for exactly that situation.

pub struct Steps<T: Transposer + 'static> {
    steps:             VecDeque<StepWrapper<T>>,
    num_deleted_steps: usize,
}

impl<T: Transposer + Clone> Steps<T> {
    pub fn new(transposer: T, pre_init_step: PreInitStep<T>, start_time: T::Time, rng_seed: [u8; 32]) -> Result<Self, T> {
        let mut steps = VecDeque::new();
        steps.push_back(StepWrapper::new_init(transposer, pre_init_step, start_time, rng_seed)?);
        Ok(Self {
            steps,
            num_deleted_steps: 0,
        })
    }

    pub fn poll(
        &mut self,
        time: T::Time,
        input_buffer: &mut BTreeSet<BoxedInput<'static, T, ArcTK>>,
    ) -> StepsPoll<T> {
        todo!()
    }

    pub fn try_poll_shared(&self, time: T::Time) -> Option<Interpolation<T, ArcTK>> {
        todo!()
    }

    pub fn rollback(
        &mut self,
        time: T::Time,
        input_buffer: &mut BTreeSet<BoxedInput<'static, T, ArcTK>>,
    ) -> StepsRollback<T> {
        todo!()
    }

    pub fn finalize(&mut self, time: T::Time) {
        todo!()
    }

    pub fn advance(&mut self, time: T::Time) {
        todo!()
    }

    fn get_by_sequence_number(&self, i: usize) -> Option<&StepWrapper<T>> {
        let i = i.checked_sub(self.num_deleted_steps)?;

        self.steps.get(i)
    }

    fn get_mut_by_sequence_number(&mut self, i: usize) -> Option<&mut StepWrapper<T>> {
        let i = i.checked_sub(self.num_deleted_steps)?;

        self.steps.get_mut(i)
    }

    fn get_last(&self) -> &StepWrapper<T> {
        self.steps.back().unwrap()
    }

    fn get_last_mut(&mut self) -> &mut StepWrapper<T> {
        self.steps.back_mut().unwrap()
    }

    fn get_before_or_at(&mut self, time: T::Time) -> Result<BeforeStatus<'_, T>, ()> {
        // this is just mimicking partition_point, because vecdeque isn't actually contiguous
        let mut i = match self
            .steps
            .binary_search_by_key(&time, |s| s.step.get_time())
        {
            Ok(i) => i,
            Err(i) => i.checked_sub(1).ok_or(())?,
        };

        // this is only indexed into in two places. here and in the loop.
        let steps = unsafe { Into::<std::ptr::NonNull<_>>::into(&mut self.steps).as_mut() };
        let mut step_i = steps.get_mut(i).ok_or(())?;
        if step_i.step.is_saturated() {
            // SAFETY: This line can be deleted with polonius
            let step_i = unsafe { Into::<std::ptr::NonNull<_>>::into(step_i).as_ref() };
            return Ok(BeforeStatus::SaturatedImmediate(step_i))
        }

        let mut step_next;

        i = i.checked_sub(1).ok_or(())?;
        while i > 0 {
            step_next = step_i;
            // this is only indexed into in two places. here and at the declaration of step_i.
            let steps = unsafe { Into::<std::ptr::NonNull<_>>::into(&mut self.steps).as_mut() };
            step_i = steps.get_mut(i).ok_or(())?;
            if step_i.step.is_unsaturated() {
                i -= 1;
                continue
            }

            if step_i.step.is_saturating() {
                // SAFETY: This line can be deleted with polonius
                let step_i = unsafe { Into::<std::ptr::NonNull<_>>::into(step_i).as_mut() };
                return Ok(BeforeStatus::Saturating(step_i))
            }

            // SAFETY: This line can be deleted with polonius
            let step_i = unsafe { Into::<std::ptr::NonNull<_>>::into(step_i).as_mut() };
            return Ok(BeforeStatus::SaturatedDistant(step_i, step_next))
        }

        Err(())
    }

    pub fn delete_before(&mut self, time: T::Time) {}
}

pub struct StepsPoll<T: Transposer> {
    completed_steps: Option<RangeInclusive<usize>>,
    result:          StepsPollResult<T>,
}

pub enum StepsPollResult<T: Transposer> {
    Ready(Interpolation<T, ArcTK>),
    Pending(/* step_id */ usize),
    NeedsState(/* step_id */ usize),
    Event(/* step_id */ usize, T::Time, T::OutputEvent),
}

pub struct StepsRollback<T: Transposer> {
    rollback_steps: Option<Range</* step_id */ usize>>,
    rollback_time:  Option<T::Time>,
}

pub struct StepWrapper<T: Transposer + 'static> {
    pub step:             Step<'static, T, ArcTK>,
    pub first_emitted_id: Option<usize>,
}

impl<T: Transposer + Clone> StepWrapper<T> {
    pub fn new_init(transposer: T, pre_init_step: PreInitStep<T>, start_time: T::Time, rng_seed: [u8; 32]) -> Result<Self, T> {
        Ok(Self {
            step:             Step::new_init(transposer, pre_init_step, start_time, rng_seed)?,
            first_emitted_id: None,
        })
    }
}

pub enum BeforeStatus<'a, T: Transposer+ 'static> {
    SaturatedImmediate(&'a StepWrapper<T>),
    SaturatedDistant(&'a mut StepWrapper<T>, &'a mut StepWrapper<T>),
    Saturating(&'a mut StepWrapper<T>),
}
