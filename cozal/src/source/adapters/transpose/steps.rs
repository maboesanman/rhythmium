use anyhow::Result;
use archery::ArcTK;
use std::collections::VecDeque;

use crate::transposer::step::{BoxedInput, Interpolation, PreInitStep, Step};
use crate::transposer::Transposer;

// a collection of Rc which are guranteed not to be cloned outside the collection is Send
// whenever the same collection, but with Arc would be Send, so we do an unsafe impl for exactly that situation.

pub struct StepList<T: Transposer + 'static> {
    steps: VecDeque<StepWrapper<T>>,
    pub next_step_uuid: u64,
    num_deleted_steps: usize,
}

impl<T: Transposer + Clone> StepList<T> {
    pub fn new(
        transposer: T,
        pre_init_step: PreInitStep<T>,
        start_time: T::Time,
        rng_seed: [u8; 32],
    ) -> Result<Self, T> {
        let mut steps = VecDeque::new();
        steps.push_back(StepWrapper::new_init(
            transposer,
            pre_init_step,
            start_time,
            rng_seed,
        )?);
        Ok(Self {
            steps,
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

    pub fn get_last_two_steps(&mut self) -> Option<(&mut StepWrapper<T>, &mut StepWrapper<T>)> {
        get_last_two_mut(&mut self.steps)
    }

    pub fn create_interpolation(&self, time: T::Time) -> Interpolation<T, ArcTK> {
        let i = self
            .steps
            .partition_point(|s| s.step.get_time() <= time)
            .checked_sub(1)
            .unwrap();

        self.steps.get(i).unwrap().step.interpolate(time).unwrap()
    }

    pub fn get_first_possible_event_emit_time(&self) -> Option<T::Time> {
        let mut value = None;
        for step_wrapper in self.steps.iter().rev() {
            if step_wrapper.step.can_produce_events() {
                break;
            } else {
                value = Some(step_wrapper.step.get_time())
            }
        }

        value
    }

    pub fn delete_at_or_after(
        &mut self,
        time: T::Time,
    ) -> impl '_ + IntoIterator<Item = BoxedInput<'static, T, ArcTK>> {
        let i = self.steps.partition_point(|s| s.step.get_time() < time);
        self.steps.drain(i..).flat_map(|s| s.step.drain_inputs())
    }
}

pub struct StepWrapper<T: Transposer + 'static> {
    pub uuid: u64,
    pub step: Step<'static, T, ArcTK>,
}

impl<T: Transposer + Clone> StepWrapper<T> {
    pub fn new_init(
        transposer: T,
        pre_init_step: PreInitStep<T>,
        start_time: T::Time,
        rng_seed: [u8; 32],
    ) -> Result<Self, T> {
        Ok(Self::new(
            Step::new_init(transposer, pre_init_step, start_time, rng_seed)?,
            0,
        ))
    }

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
