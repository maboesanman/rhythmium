mod expire_handle_factory;
mod interpolate_context;
// mod interpolation;
mod sub_step;
mod sub_step_update_context;
mod time;
mod transposer_metadata;
mod wrapped_transposer;

// #[cfg(test)]
// mod test;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Waker};
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::task::Poll;

use archery::{ArcTK, SharedPointer, SharedPointerKind};
use futures_channel::{mpsc, oneshot};
use futures_util::{FutureExt, StreamExt};
use smallvec::SmallVec;
use sub_step::SubStep;
use time::ScheduledTime;
use wrapped_transposer::WrappedTransposer;

use crate::transposer::Transposer;

use super::input_state_manager::InputStateManager;
use super::output_event_manager::OutputEventManager;

pub struct Step<T: Transposer, P: SharedPointerKind = ArcTK> {
    // status: StepStatus<T, P>,
    // data: SharedPointer<StepData<T, P, S>, P>,
    steps: Vec<Box<dyn SubStep<T, P>>>,

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

pub struct NoInput<T>(InputStateManager<T>);

impl<T> Default for NoInput<T> {
    fn default() -> Self {
        Self(InputStateManager::default())
    }
}

impl<T: Transposer, P: SharedPointerKind> Drop for Step<T, P> {
    fn drop(&mut self) {
        Self::drop_input_state(self.shared_step_state);
    }
}

impl<T: Transposer, P: SharedPointerKind> Step<T, P> {
    fn drop_input_state(ptr: NonNull<(OutputEventManager<T>, InputStateManager<T>)>) {
        unsafe { NonNull::drop_in_place(ptr) }
    }
}

impl<T: Transposer, P: SharedPointerKind> Step<T, P> {
    fn new_input_state() -> NonNull<(OutputEventManager<T>, InputStateManager<T>)> {
        let input_state = Box::new(Default::default());
        NonNull::from(Box::leak(input_state))
    }
}

impl<T: Transposer, P: SharedPointerKind> Step<T, P> {
    pub fn get_input_state_mut(&mut self) -> &mut InputStateManager<T> {
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
        // let input_state = Self::new_input_state();
        // let (output_sender, output_reciever) = mpsc::channel(1);
        // let future =
        //     WrappedTransposer::<T, P>::init::<S>(transposer, rng_seed, start_time, input_state);
        // let future: SaturationFuture<'_, T, P> = Box::pin(future);
        // // SAFETY: the future here can only hold things that the step is already generic over and contains.
        // // this means that this lifetime forging to 'static is ok.
        // let future: SaturationFuture<'static, T, P> = unsafe { core::mem::transmute(future) };

        // let status = StepStatus::Saturating {
        //     future,
        //     output_reciever,
        // };

        // Step {
        //     data: SharedPointer::new(StepData::Init(start_time)),
        //     input_state,
        //     status,
        //     event_count: 0,
        //     can_produce_events: true,

        //     #[cfg(debug_assertions)]
        //     uuid_self: uuid::Uuid::new_v4(),
        //     #[cfg(debug_assertions)]
        //     uuid_prev: None,
        // }

        todo!()
    }

    // pub fn next_unsaturated(
    //     &self,
    //     next_inputs: &mut Option<StepInputs<T, P>>,
    // ) -> Result<Option<Self>, NextUnsaturatedErr> {
    //     let wrapped_transposer = match &self.status {
    //         StepStatus::Saturated { wrapped_transposer } => wrapped_transposer,
    //         _ => return Err(NextUnsaturatedErr::NotSaturated),
    //     };

    //     let next_scheduled_time = wrapped_transposer.metadata.get_next_scheduled_time();
    //     let next_inputs_time = next_inputs.as_ref().map(|i| i.time);
    //     let data = match (next_inputs_time, next_scheduled_time) {
    //         (None, None) => return Ok(None),
    //         (None, Some(t)) => StepData::Scheduled(*t),
    //         (Some(_), None) => StepData::Input(core::mem::take(next_inputs).unwrap()),
    //         (Some(i_t), Some(s_t)) => {
    //             if i_t > s_t.time {
    //                 StepData::Scheduled(*s_t)
    //             } else {
    //                 StepData::Input(core::mem::take(next_inputs).unwrap())
    //             }
    //         }
    //     };

    //     Ok(Some(Self {
    //         data: SharedPointer::new(data),
    //         input_state: Self::new_input_state(),
    //         status: StepStatus::Unsaturated,
    //         event_count: 0,
    //         can_produce_events: true,

    //         #[cfg(debug_assertions)]
    //         uuid_self: uuid::Uuid::new_v4(),
    //         #[cfg(debug_assertions)]
    //         uuid_prev: Some(self.uuid_self),
    //     }))
    // }

    // pub fn next_scheduled_unsaturated(&self) -> Result<Option<Self>, NextUnsaturatedErr> {
    //     self.next_unsaturated(&mut None)
    // }

    pub fn saturate_take(&mut self, prev: &mut Self) -> Result<(), SaturateTakeErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != Some(prev.uuid_self) {
            return Err(SaturateTakeErr::IncorrectPrevious);
        }

        let wrapped_transposer = prev.take()?;

        self.saturate(wrapped_transposer);

        Ok(())
    }

    pub fn saturate_clone(&mut self, prev: &Self) -> Result<(), SaturateCloneErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != Some(prev.uuid_self) {
            return Err(SaturateCloneErr::IncorrectPrevious);
        }

        let wrapped_transposer = prev.clone()?;

        self.saturate(wrapped_transposer);

        Ok(())
    }

    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateTakeErr> {
        todo!()
    }

    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateCloneErr>
    where
        T: Clone,
    {
        todo!()
    }

    fn saturate(&mut self, wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>)
    where
        T: Clone,
    {
        todo!()
    }

    pub fn desaturate(&mut self) {
        todo!()
    }

    pub fn poll(&mut self, waker: &Waker) -> Result<StepPoll<T>, PollErr> {
        todo!()
    }

    // pub fn interpolate(
    //     &self,
    //     time: T::Time,
    // ) -> Result<impl Interpolation<T>, InterpolateErr> {
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
        todo!()
    }

    pub fn is_unsaturated(&self) -> bool {
        todo!()
    }

    pub fn is_saturating(&self) -> bool {
        todo!()
    }

    pub fn is_saturated(&self) -> bool {
        todo!()
    }

    pub fn can_produce_events(&self) -> bool {
        self.can_produce_events
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StepPoll<T: Transposer> {
    Emitted(T::OutputEvent),
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
pub enum SaturateTakeErr {
    PreviousNotSaturated,
    SelfNotUnsaturated,
    #[cfg(debug_assertions)]
    IncorrectPrevious,
    PreviousHasActiveInterpolations,
}

#[derive(Debug)]
pub enum SaturateCloneErr {
    PreviousNotSaturated,
    SelfNotUnsaturated,
    #[cfg(debug_assertions)]
    IncorrectPrevious,
}
