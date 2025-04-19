use std::{
    collections::{BTreeSet, VecDeque},
    ops::Bound,
    task::Waker,
};

use archery::ArcTK;

use crate::{
    source::source_poll::{Interrupt, LowerBound, SourceBound, UpperBound},
    transposer::{
        input_erasure::{ErasedInput, ErasedInputState}, step::{BoxedInput, InitStep, Interpolation, PossiblyInitStep, PreInitStep, Step, StepPoll}, Transposer
    },
};

pub struct WorkingTimelineSlice<T: Transposer + 'static> {
    init_step: Option<Box<InitStep<T, ArcTK>>>,
    steps: VecDeque<StepWrapper<T>>,
    next_step_uuid: u64,
    num_deleted_steps: usize,
    input_buffer: BTreeSet<BoxedInput<'static, T, ArcTK>>,

    poll_lower_bound: LowerBound<T::Time>,
    interrupt_lower_bound: LowerBound<T::Time>,
    interrupt_upper_bound: UpperBound<T::Time>,
}

impl<T: Transposer + Clone> WorkingTimelineSlice<T> {}

struct StepWrapper<T: Transposer + 'static> {
    uuid: u64,
    step: Step<'static, T, ArcTK>,
}

impl<T: Transposer + Clone> WorkingTimelineSlice<T> {
    /// Create a new Timeline with only a pre init step.
    pub fn new(
        transposer: T,
        pre_init_step: PreInitStep<T>,
        rng_seed: [u8; 32],
    ) -> Result<Self, T> {
        Ok(Self {
            init_step: Some(Box::new(InitStep::new(
                transposer,
                pre_init_step,
                rng_seed,
            )?)),
            steps: VecDeque::new(),
            next_step_uuid: 1,
            num_deleted_steps: 0,
            input_buffer: BTreeSet::new(),

            poll_lower_bound: LowerBound::min(),
            interrupt_lower_bound: LowerBound::min(),
            interrupt_upper_bound: UpperBound::min(),
        })
    }

    fn advance_min_lower_bound(&mut self, new_min: LowerBound<T::Time>) {
        if new_min <= self.poll_lower_bound.min(self.interrupt_lower_bound) {
            return;
        }

        let first_included_index = self
            .steps
            .partition_point(|s| new_min.test(&s.step.get_time()));

        let last_deleted_index = match first_included_index.checked_sub(1) {
            Some(i) => i,
            None => return
        };

        self.init_step = None;
        self.num_deleted_steps += self.steps.drain(..=last_deleted_index).count();
    }

    /// get the top uuid, and the previous, both mutably.
    /// errors when there aren't any non-init steps.
    fn get_last_two_steps(&mut self) -> Result<
        (&mut dyn PossiblyInitStep<'static, T, ArcTK>, &mut StepWrapper<T>),
        &mut dyn PossiblyInitStep<'static, T, ArcTK>
    > {
        match self.steps.len() {
            0 => Err(&mut **self.init_step.as_mut().unwrap()),
            1 => Ok((&mut **self.init_step.as_mut().unwrap(), self.steps.back_mut().unwrap())),
            _ => {
                let (a, b) = self.steps.as_mut_slices();

                let (a, b) = match b.len() {
                    0 => a.split_at_mut(a.len() - 1),
                    1 => (a, b),
                    _ => b.split_at_mut(b.len() - 1),
                };

                Ok((
                    &mut a.last_mut().unwrap().step,
                    b.last_mut().unwrap(),
                ))
            }
        }
    }

    /// get the uuid of the last step.
    fn top_uuid(&self) -> u64 {
        self.steps.back().map(|s| s.uuid).unwrap_or(0)
    }

    /// Advance the lower bound that interpolates may be created by.
    pub fn advance_poll_lower_bound(&mut self, poll_lower_bound: LowerBound<T::Time>) {
        let new_min = poll_lower_bound.min(self.interrupt_lower_bound);
        self.advance_min_lower_bound(new_min);
        self.poll_lower_bound = poll_lower_bound;
    }

    /// Advance the lower bound that new interrupts may be produced in.
    pub fn advance_interrupt_lower_bound(&mut self, interrupt_lower_bound: LowerBound<T::Time>) {
        let new_min = interrupt_lower_bound.min(self.poll_lower_bound);
        self.advance_min_lower_bound(new_min);
        self.interrupt_lower_bound = interrupt_lower_bound;
    }

    /// Advance the upper bound to which all outgoing events must be calculated.
    /// 
    /// The poll function must populate the step chain all the way up to this upper bound
    /// (and a single unsaturated event past it, if one exists).
    pub fn advance_interrupt_upper_bound(&mut self, interrupt_upper_bound: UpperBound<T::Time>) {
        self.interrupt_upper_bound = interrupt_upper_bound;
    }

    /// process the given interrupt.
    pub fn handle_interrupt(
        &mut self,
        input_hash: u64,
        time: T::Time,
        interrupt: Interrupt<BoxedInput<'static, T, ArcTK>>,
    ) -> Option<T::Time> {
        let first_delete = self.steps.partition_point(|s| time <= s.step.get_time());

        match interrupt {
            Interrupt::Event(e) => {
                self.input_buffer.extend(
                    self.steps
                        .drain(first_delete..)
                        .flat_map(|step| step.step.drain_inputs())
                        .chain(Some(e)),
                );
            }
            Interrupt::Rollback => {
                self.input_buffer.extend(
                    self.steps
                        .drain(first_delete..)
                        .flat_map(|step| step.step.drain_inputs())
                        .filter(|i| i.get_input_hash() != input_hash),
                );
            }
        }

        todo!()
    }

    /// poll work on the steps, which is only complete when the next step is after the
    /// interrupt_upper_bound, or there are no more steps available.
    pub fn poll<F>(&mut self, mut interrupt_waker_fn: F) -> WorkingTimelineSlicePoll<T>  where F: FnMut(u64) -> Waker {
        // assume the top step is never saturated unless there are no remaining events.
        loop {
            let top_step_uuid = self.top_uuid();

            let top_step: &mut dyn PossiblyInitStep<_, _> = match self.steps.back_mut() {
                Some(s) => &mut s.step,
                None => &mut **self.init_step.as_mut().unwrap(),
            };

            if top_step.is_unsaturated() {
                if !self.interrupt_upper_bound.test(&top_step.get_time()) {
                    return WorkingTimelineSlicePoll::Ready { next_time: Some(top_step.get_time()) }
                }

                let (prev, curr) = match self.get_last_two_steps() {
                    Ok(x) => x,
                    Err(_) => unreachable!(),
                };

                curr.step.start_saturate_clone(prev).unwrap();

                continue;
            }

            if top_step.is_saturating() {
                let top_step_waker = interrupt_waker_fn(top_step_uuid);
                match top_step.poll(&top_step_waker).unwrap() {
                    StepPoll::Emitted(event) => return WorkingTimelineSlicePoll::Emitted {
                        time: top_step.get_time(),
                        event
                    },
                    StepPoll::StateRequested(input) => return WorkingTimelineSlicePoll::StateRequested {
                        time: top_step.get_time(),
                        input,
                        step_uuid: self.top_uuid()
                    },
                    StepPoll::Pending => return WorkingTimelineSlicePoll::Pending {
                        step_uuid: self.top_uuid()
                    },
                    StepPoll::Ready => {},
                }

                continue;
            }

            if top_step.is_saturated() {
                let step = match top_step.next_unsaturated(&mut self.input_buffer).unwrap() {
                    Some(step) => step,
                    None => return WorkingTimelineSlicePoll::Ready { next_time: None },
                };
                self.steps.push_back(StepWrapper {
                    uuid: self.next_step_uuid,
                    step,
                });
                self.next_step_uuid += 1;

                continue;
            }
        }
    }

    /// provide a requested input state to the requesting step.
    pub fn provide_input_state(
        &mut self,
        uuid: u64,
        state: Box<ErasedInputState<T>>,
    ) -> Result<(), Box<ErasedInputState<T>>> {
        use std::cmp::Ordering;

        for step in self.steps.iter_mut().rev() {
            match step.uuid.cmp(&uuid) {
                Ordering::Less => return Err(state),
                Ordering::Equal => return step.step.provide_input_state(state),
                Ordering::Greater => continue,
            }
        }

        Err(state)
    }

    /// produce an interpolation for the given time.
    pub fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, ArcTK>, ()> {

        // TODO: THIS SEEMS LIKE THE EQUALS CASE IS WRONG
        match self.steps.partition_point(|s| s.step.get_time() <= time).checked_sub(1) {
            Some(i) => self.steps.get(i).unwrap().step.interpolate(time),
            None => self.init_step.as_ref().ok_or(())?.interpolate(time),
        }.map_err(|_| ())
    }

    /// the lower bound for times that a step might request state
    pub fn tentative_state_and_event_lower_bound(&self) -> LowerBound<T::Time> {
        match self.steps.back() {
            Some(last) => {
                if last.step.can_produce_events() {
                    return LowerBound::inclusive(last.step.get_time())
                }

                return LowerBound::max()
            },
            None => LowerBound::min(),
        }
    }
}

pub enum WorkingTimelineSlicePoll<T: Transposer> {
    Emitted {
        time: T::Time,
        event: T::OutputEvent,
    },
    StateRequested {
        time: T::Time,
        input: Box<ErasedInput<T>>,
        step_uuid: u64,
    },
    Ready {
        next_time: Option<T::Time>,
    },
    Pending {
        step_uuid: u64,
    },
}
