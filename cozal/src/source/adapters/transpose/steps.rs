use anyhow::Result;
use archery::ArcTK;
use std::collections::VecDeque;

use crate::source::source_poll::LowerBound;
use crate::transposer::Transposer;
use crate::transposer::step::{InitStep, Interpolation, PreInitStep, PreviousStep, Step};

// a collection of Rc which are guranteed not to be cloned outside the collection is Send
// whenever the same collection, but with Arc would be Send, so we do an unsafe impl for exactly that situation.

pub struct StepList<T: Transposer + 'static> {
    init_step: InitStep<T, ArcTK>,
    steps: VecDeque<StepWrapper<T>>,
    pub next_step_uuid: u64,
    num_deleted_steps: usize,
}

impl<T: Transposer + Clone> StepList<T> {
    pub fn new(
        transposer: T,
        pre_init_step: PreInitStep<T>,
        rng_seed: [u8; 32],
    ) -> Result<Self, T> {
        Ok(Self {
            init_step: InitStep::new(transposer, pre_init_step, rng_seed)?,
            steps: VecDeque::new(),
            next_step_uuid: 1,
            num_deleted_steps: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    fn get_step_index_by_uuid(&self, uuid: u64) -> Option<usize> {
        self.steps.binary_search_by_key(&uuid, |w| w.uuid).ok()
    }

    pub fn get_step_wrapper_mut_by_uuid(&mut self, uuid: u64) -> Option<&mut StepWrapper<T>> {
        self.steps.get_mut(self.get_step_index_by_uuid(uuid)?)
    }

    pub fn get_init_step_mut(&mut self) -> &mut InitStep<T> {
        &mut self.init_step
    }

    pub fn get_last_step(&self) -> &StepWrapper<T> {
        self.steps.back().unwrap()
    }

    pub fn push_step(&mut self, step: Step<'static, T, ArcTK>) {
        self.steps.push_back(StepWrapper {
            uuid: self.next_step_uuid,
            step,
        });
        self.next_step_uuid += 1;
    }

    pub fn get_last_two_steps(
        &mut self,
    ) -> Option<(&mut dyn PreviousStep<T, ArcTK>, &mut StepWrapper<T>)> {
        match self.steps.len() {
            0 => None,
            1 => Some((&mut self.init_step, self.steps.back_mut()?)),
            _ => {
                let (a, b) = get_last_two_mut(&mut self.steps)?;
                Some((&mut a.step, b))
            }
        }
    }

    pub fn create_interpolation(&self, time: T::Time) -> Interpolation<T, ArcTK> {
        let i = self
            .steps
            .partition_point(|s| s.step.get_time() <= time)
            .checked_sub(1)
            .unwrap();

        self.steps.get(i).unwrap().step.interpolate(time).unwrap()
    }

    pub fn delete_outside_lower_bound(
        &mut self,
        lower_bound: LowerBound<T::Time>,
    ) -> impl '_ + IntoIterator<Item = Step<'static, T, ArcTK>> {
        let i = self
            .steps
            .partition_point(|s| !lower_bound.test(&s.step.get_time()));
        self.steps.drain(i..).map(|w| w.step)
    }

    pub fn earliest_possible_event_time(&self) -> Option<T::Time> {
        let last_step = &self.get_last_step().step;
        if last_step.can_produce_events() {
            Some(last_step.get_time())
        } else {
            None
        }
    }

    // the lower bound for where interrupts may come from, if coming from a step.
    pub fn get_finalize_bound(&self) -> LowerBound<T::Time> {
        let last_step = &self.get_last_step().step;
        if last_step.can_produce_events() {
            LowerBound::inclusive(last_step.get_time())
        } else {
            LowerBound::max()
        }
    }
}

pub struct StepWrapper<T: Transposer + 'static> {
    pub uuid: u64,
    pub step: Step<'static, T, ArcTK>,
}

impl<T: Transposer + Clone> StepWrapper<T> {
    pub fn new(step: Step<'static, T, ArcTK>, uuid: u64) -> Self {
        Self { uuid, step }
    }
}

fn get_last_two_mut<T>(deque: &mut VecDeque<T>) -> Option<(&mut T, &mut T)> {
    let i = deque.len().checked_sub(2)?;
    let (a, b) = get_adjacent_mut(deque, i)?;
    Some((a, b?))
}

fn get_adjacent_mut<T>(deque: &mut VecDeque<T>, i: usize) -> Option<(&mut T, Option<&mut T>)> {
    let len = deque.len();
    if i >= len {
        return None;
    }

    let (front, back) = deque.as_mut_slices();
    let front_len = front.len();

    if i < front_len {
        // First element is in the front slice.
        if i + 1 < front_len {
            // Both elements are in the front slice.
            // We can safely split this slice to get two distinct mutable references.
            let (left, right) = front.split_at_mut(i + 1);
            // left[i] is the element at index i and right[0] is at index i+1.
            Some((&mut left[i], right.get_mut(0)))
        } else {
            // i is in the front slice and i+1 is in the back slice.
            Some((&mut front[i], back.get_mut(0)))
        }
    } else {
        // First element is in the back slice.
        let j = i - front_len;
        if j + 1 < back.len() {
            // Both elements are in the back slice.
            let (left, right) = back.split_at_mut(j + 1);
            Some((&mut left[j], right.get_mut(0)))
        } else {
            // i is in back and there is no next element.
            Some((&mut back[j], None))
        }
    }
}
