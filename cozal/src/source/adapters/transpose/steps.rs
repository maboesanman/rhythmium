use anyhow::Result;
use archery::ArcTK;
use std::collections::{BTreeSet, VecDeque};
use std::ops::Range;
use std::task::Waker;

use crate::transposer::input_erasure::{ErasedInput, ErasedInputState};
use crate::transposer::step::{BoxedInput, Interpolation, PreInitStep, Step, StepPoll};
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

    fn get_step_index_by_uuid(&self, uuid: u64) -> Option<usize> {
        self.steps.binary_search_by_key(&uuid, |w| w.uuid).ok()
    }

    pub fn get_step_wrapper_by_uuid(&self, uuid: u64) -> Option<&StepWrapper<T>> {
        self.steps.get(self.get_step_index_by_uuid(uuid)?)
    }

    pub fn get_step_wrapper_mut_by_uuid(&mut self, uuid: u64) -> Option<&mut StepWrapper<T>> {
        self.steps.get_mut(self.get_step_index_by_uuid(uuid)?)
    }

    pub fn get_step_saturated_and_next_mut(&mut self, uuid: u64) -> Option<(&mut StepWrapper<T>, Option<&mut StepWrapper<T>>)> {
        let index = self.get_step_index_by_uuid(uuid)?;
        get_adjacent_mut(&mut self.steps, index)
    }

    pub fn get_last_step(&self) -> &Step<T, ArcTK> {
        &self.steps.back().unwrap().step
    }

    pub fn push_step(&mut self, step: Step<'static, T, ArcTK>) -> &mut StepWrapper<T> {
        self.steps.push_back(StepWrapper {
            uuid: self.next_step_uuid,
            step,
        });
        self.next_step_uuid += 1;
        self.steps.back_mut().unwrap()
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
                break
            } else {
                value = Some(step_wrapper.step.get_time())
            }
        }

        value
    }

    /// poll the step list until the step preceding a certain time is saturated.
    pub fn prepare_poll<'a, 'b>(
        &'a mut self,
        time: T::Time,
        input_buffer: &'b mut BTreeSet<BoxedInput<'static, T, ArcTK>>,
        interrupt_waker: Waker,
    ) -> Result<StepListPollResult<'a, T>> {
        // i is the index of the last saturated step before or at time.
        let mut i = self
            .steps
            .partition_point(|s| ((!s.step.is_unsaturated()) && s.step.get_time() <= time))
            .checked_sub(1)
            .ok_or(anyhow::format_err!("bad step list"))?;

        let prepared_i = loop {
            let (curr, next) =
                get_adjacent_mut(&mut self.steps, i).ok_or(anyhow::format_err!("bad step list"))?;
            let curr_saturated = curr.step.is_saturated();

            if next.is_none() && curr_saturated {
                if let Some(new_next) = curr.step.next_unsaturated(input_buffer).map_err(|_| anyhow::format_err!("bad step list"))? {
                    self.steps.push_back(StepWrapper::new(new_next, self.next_step_uuid));
                    self.next_step_uuid += 1;
                    continue;
                }
            }

            match (curr_saturated, next) {
                (false, _) => {
                    let result = match curr.step.poll(&interrupt_waker).map_err(|_| anyhow::format_err!("bad step list"))? {
                        StepPoll::Emitted(output_event) => {
                            StepListPollResult::Event { time: curr.step.get_time(), event: output_event }
                        },
                        StepPoll::StateRequested(input) => {
                            let step_id = curr.step.get_sequence_number();
                            StepListPollResult::NeedsState { step_id, time: curr.step.get_time(), input }
                        },
                        StepPoll::Pending => StepListPollResult::Pending,
                        StepPoll::Ready => continue,
                    };
                    return Ok(result);
                }
                (true, None) => {
                    break i;
                },
                (true, Some(s)) => {
                    if s.step.get_time() > time {
                        break i;
                    }

                    s.step.start_saturate_clone(&curr.step).map_err(|_| anyhow::format_err!("bad step list"))?;
                    i += 1;
                },
            }
        };

        Ok(StepListPollResult::Ready { preceeding_step: &self.steps[prepared_i].step })
    }

    pub fn provide_state(
        &mut self,
        step_id: usize,
        state: Box<ErasedInputState<T>>,
    ) -> Result<(), ()> {
        let step = self.get_mut_by_sequence_number(step_id).ok_or(())?;
        step.step.provide_input_state(state).map_err(|_| ())?;
        Ok(())
    }

    fn get_by_sequence_number(&self, sequence_number: usize) -> Option<&StepWrapper<T>> {
        self.steps.get(sequence_number.checked_sub(self.num_deleted_steps)?)
    }

    fn get_mut_by_sequence_number(&mut self, sequence_number: usize) -> Option<&mut StepWrapper<T>> {
        self.steps.get_mut(sequence_number.checked_sub(self.num_deleted_steps)?)
    }

    pub fn delete_before(&mut self, time: T::Time) {
        let i = self.steps.partition_point(|s| s.step.get_time() < time);
        self.num_deleted_steps += self.steps.drain(..i).count();
    }

    pub fn delete_at_or_after(&mut self, time: T::Time) -> impl '_ + IntoIterator<Item = BoxedInput<'static, T, ArcTK>> {
        let i = self.steps.partition_point(|s| s.step.get_time() < time);
        self.steps.drain(i..).flat_map(|s| s.step.drain_inputs())
    }
}

// pub struct StepListPoll<T: Transposer> {
//     completed_steps: Option<RangeInclusive<usize>>,
//     result: StepListPollResult<T>,
// }

pub enum StepListPollResult<'a, T: Transposer> {
    Ready {
        preceeding_step: &'a Step<'a, T, ArcTK>,

        // there is no passed 'next time' because it may be far from this step. it isn't simply the time of the next step
        // after this one. it is the first saturating or unsaturated step marked with "may produce events" in the whole collection.
    },
    Pending,
    NeedsState {
        step_id: usize,
        time: T::Time,
        input: Box<ErasedInput<T>>,
    },
    Event {
        time: T::Time,
        event: T::OutputEvent,
    },
}

pub enum StepListPollEventsResult<T: Transposer> {
    Ready,
    Pending,
    NeedsState(/* step_id */ usize, T::Time, Box<ErasedInput<T>>),
    Event(T::Time, T::OutputEvent),
}

pub struct StepListRollback<T: Transposer> {
    rollback_steps: Option<Range</* step_id */ usize>>,
    rollback_time: Option<T::Time>,
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
        Ok(Self::new(Step::new_init(
            transposer,
            pre_init_step,
            start_time,
            rng_seed,
        )?, 0))
    }

    pub fn new(step: Step<'static, T, ArcTK>, uuid: u64) -> Self {
        Self {
            uuid,
            step,
        }
    }
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
            return Some((&mut left[i], right.get_mut(0)));
        } else {
            // i is in the front slice and i+1 is in the back slice.
            return Some((&mut front[i], back.get_mut(0)));
        }
    } else {
        // First element is in the back slice.
        let j = i - front_len;
        if j + 1 < back.len() {
            // Both elements are in the back slice.
            let (left, right) = back.split_at_mut(j + 1);
            return Some((&mut left[j], right.get_mut(0)));
        } else {
            // i is in back and there is no next element.
            return Some((&mut back[j], None));
        }
    }
}
