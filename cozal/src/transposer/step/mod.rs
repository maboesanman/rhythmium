mod expire_handle_factory;
mod interpolate_context;
mod interpolation;
mod step_inputs;
mod sub_step_update_context;
mod time;
mod transposer_metadata;
mod wrapped_transposer;

#[cfg(test)]
mod test;

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Waker};

use archery::{ArcTK, SharedPointer, SharedPointerKind};
use futures_channel::{mpsc, oneshot};
use futures_util::{FutureExt, StreamExt};
pub use interpolation::Interpolation;
use step_inputs::StepInputs;
use time::ScheduledTime;
use wrapped_transposer::WrappedTransposer;

use crate::transposer::Transposer;

enum StepData<T: Transposer, P: SharedPointerKind> {
    Init(T::Time),
    Input(StepInputs<T, P>),
    Scheduled(ScheduledTime<T::Time>),
}

type SaturationFuture<'a, T, P> =
    Pin<Box<dyn 'a + Future<Output = SharedPointer<WrappedTransposer<T, P>, P>>>>;

enum StepStatus<T: Transposer, P: SharedPointerKind> {
    Unsaturated,
    Saturating {
        future: SaturationFuture<'static, T, P>,
        output_reciever: mpsc::Receiver<(T::OutputEvent, oneshot::Sender<()>)>,
    },
    Saturated {
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    },
}

impl<T: Transposer, P: SharedPointerKind> Default for StepStatus<T, P> {
    fn default() -> Self {
        Self::Unsaturated
    }
}

pub struct Step<T: Transposer, Is: InputState<T>, P: SharedPointerKind = ArcTK> {
    data: SharedPointer<StepData<T, P>, P>,
    input_state: SharedPointer<Is, P>,
    status: StepStatus<T, P>,
    event_count: usize,
    can_produce_events: bool,

    #[cfg(debug_assertions)]
    uuid_self: uuid::Uuid,
    #[cfg(debug_assertions)]
    uuid_prev: Option<uuid::Uuid>,
}

/// this type holds the lazy state values for all inputs.
/// all the lazy population logic is left to the instantiator of step.
pub trait InputState<T: Transposer> {
    fn new() -> Self;
    fn get_provider(&self) -> &T::InputStateManager;
}

pub struct NoInput;
pub struct NoInputManager;

impl<T: Transposer<InputStateManager = NoInputManager>> InputState<T> for NoInput {
    fn new() -> Self {
        NoInput
    }

    fn get_provider(&self) -> &<T as Transposer>::InputStateManager {
        &NoInputManager
    }
}

impl<T: Transposer, Is: InputState<T>, P: SharedPointerKind> Drop for Step<T, Is, P> {
    fn drop(&mut self) {
        let status = core::mem::replace(&mut self.status, StepStatus::Unsaturated);

        match status {
            StepStatus::Unsaturated => {}
            StepStatus::Saturating {
                future,
                output_reciever: _,
            } => {
                let future: SaturationFuture<'static, T, P> = future;
                // SAFETY: the future here can only hold things that the step is already generic over and contains.
                // this means that this lifetime forging to 'static is ok.
                let future: SaturationFuture<'_, T, P> = unsafe { core::mem::transmute(future) };

                drop(future);
            }
            StepStatus::Saturated {
                wrapped_transposer: _,
            } => {}
        }
    }
}

impl<T: Transposer, Is: InputState<T>, P: SharedPointerKind> Step<T, Is, P> {
    pub fn new_init(transposer: T, start_time: T::Time, rng_seed: [u8; 32]) -> Self {
        let input_state = SharedPointer::new(Is::new());
        let (output_sender, output_reciever) = mpsc::channel(1);
        let future = WrappedTransposer::<T, P>::init::<Is>(
            transposer,
            rng_seed,
            start_time,
            input_state.clone(),
            0,
            output_sender,
        );
        let future: SaturationFuture<'_, T, P> = Box::pin(future);
        // SAFETY: the future here can only hold things that the step is already generic over and contains.
        // this means that this lifetime forging to 'static is ok.
        let future: SaturationFuture<'static, T, P> = unsafe { core::mem::transmute(future) };

        let status = StepStatus::Saturating {
            future,
            output_reciever,
        };

        Step {
            data: SharedPointer::new(StepData::Init(start_time)),
            input_state,
            status,
            event_count: 0,
            can_produce_events: true,

            #[cfg(debug_assertions)]
            uuid_self: uuid::Uuid::new_v4(),
            #[cfg(debug_assertions)]
            uuid_prev: None,
        }
    }

    pub fn next_unsaturated(
        &self,
        next_inputs: &mut Option<StepInputs<T, P>>,
    ) -> Result<Option<Self>, NextUnsaturatedErr> {
        let wrapped_transposer = match &self.status {
            StepStatus::Saturated { wrapped_transposer } => wrapped_transposer,
            _ => return Err(NextUnsaturatedErr::NotSaturated),
        };

        let next_scheduled_time = wrapped_transposer.metadata.get_next_scheduled_time();
        let next_inputs_time = next_inputs.as_ref().map(|i| i.time);
        let data = match (next_inputs_time, next_scheduled_time) {
            (None, None) => return Ok(None),
            (None, Some(t)) => StepData::Scheduled(*t),
            (Some(_), None) => StepData::Input(core::mem::take(next_inputs).unwrap()),
            (Some(i_t), Some(s_t)) => {
                if i_t > s_t.time {
                    StepData::Scheduled(*s_t)
                } else {
                    StepData::Input(core::mem::take(next_inputs).unwrap())
                }
            }
        };

        Ok(Some(Self {
            data: SharedPointer::new(data),
            input_state: SharedPointer::new(Is::new()),
            status: StepStatus::Unsaturated,
            event_count: 0,
            can_produce_events: true,

            #[cfg(debug_assertions)]
            uuid_self: uuid::Uuid::new_v4(),
            #[cfg(debug_assertions)]
            uuid_prev: Some(self.uuid_self),
        }))
    }

    pub fn next_scheduled_unsaturated(&self) -> Result<Option<Self>, NextUnsaturatedErr> {
        self.next_unsaturated(&mut None)
    }

    pub fn saturate_take(&mut self, prev: &mut Self) -> Result<(), SaturateTakeErr> {
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
        match core::mem::take(&mut self.status) {
            StepStatus::Saturated { wrapped_transposer } => Ok(wrapped_transposer),
            val => {
                self.status = val;
                Err(SaturateTakeErr::PreviousNotSaturated)
            }
        }
    }

    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateCloneErr>
    where
        T: Clone,
    {
        match &self.status {
            StepStatus::Saturated { wrapped_transposer } => Ok(SharedPointer::new(
                WrappedTransposer::clone(wrapped_transposer),
            )),
            _ => Err(SaturateCloneErr::PreviousNotSaturated),
        }
    }

    fn saturate(&mut self, mut wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>) {
        let (output_sender, output_reciever) = mpsc::channel(1);

        self.status = StepStatus::Saturating {
            future: match self.data.as_ref() {
                StepData::Init(_) => panic!(),
                StepData::Input(_) => {
                    let input_state = self.input_state.clone();
                    let event_count = self.event_count;
                    let step_data = self.data.clone();
                    let future: SaturationFuture<'_, T, P> = Box::pin(async move {
                        let i = match step_data.as_ref() {
                            StepData::Input(i) => i,
                            _ => unreachable!(),
                        };
                        SharedPointer::make_mut(&mut wrapped_transposer)
                            .handle_input(i, input_state, event_count, output_sender)
                            .await;
                        wrapped_transposer
                    });
                    // SAFETY: the future here can only hold things that the step is already generic over and contains.
                    // this means that this lifetime forging to 'static is ok.
                    let future: SaturationFuture<'static, T, P> =
                        unsafe { core::mem::transmute(future) };
                    future
                }
                StepData::Scheduled(t) => {
                    let t = t.time;
                    let event_count = self.event_count;
                    let input_state = self.input_state.clone();
                    let future: SaturationFuture<'_, T, P> = Box::pin(async move {
                        SharedPointer::make_mut(&mut wrapped_transposer)
                            .handle_scheduled(t, input_state, event_count, output_sender)
                            .await;
                        wrapped_transposer
                    });
                    // SAFETY: the future here can only hold things that the step is already generic over and contains.
                    // this means that this lifetime forging to 'static is ok.
                    let future: SaturationFuture<'static, T, P> =
                        unsafe { core::mem::transmute(future) };
                    future
                }
            },
            output_reciever,
        };
    }

    pub fn desaturate(&mut self) {
        self.status = StepStatus::Unsaturated;
        self.input_state = SharedPointer::new(Is::new());
    }

    pub fn poll(&mut self, waker: &Waker) -> Result<StepPoll<T>, PollErr> {
        let (future, output_reciever) = match &mut self.status {
            StepStatus::Unsaturated => return Err(PollErr::Unsaturated),
            StepStatus::Saturating {
                future,
                output_reciever,
            } => (future, output_reciever),
            StepStatus::Saturated { .. } => return Err(PollErr::Saturated),
        };

        let mut cx = Context::from_waker(waker);

        let poll = future.poll_unpin(&mut cx);

        let output = match poll {
            std::task::Poll::Ready(wrapped_transposer) => {
                self.status = StepStatus::Saturated { wrapped_transposer };
                self.can_produce_events = false;
                return Ok(StepPoll::Ready);
            }
            std::task::Poll::Pending => output_reciever.poll_next_unpin(&mut cx),
        };

        if let std::task::Poll::Ready(Some((e, sender))) = output {
            self.event_count += 1;
            let _ = sender.send(());
            return Ok(StepPoll::Emitted(e));
        }

        Ok(StepPoll::Pending)
    }

    pub fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, Is, P>, InterpolateErr> {
        let wrapped_transposer = match &self.status {
            StepStatus::Saturated { wrapped_transposer } => wrapped_transposer.clone(),
            _ => return Err(InterpolateErr::NotSaturated),
        };

        #[cfg(debug_assertions)]
        if time < wrapped_transposer.metadata.last_updated.time {
            return Err(InterpolateErr::TimePast);
        }

        Ok(Interpolation::new(time, wrapped_transposer))
    }

    pub fn get_input_state(&self) -> &Is {
        &self.input_state
    }

    pub fn get_time(&self) -> T::Time {
        match self.data.as_ref() {
            StepData::Init(time) => *time,
            StepData::Input(i) => i.time,
            StepData::Scheduled(t) => t.time,
        }
    }

    pub fn is_unsaturated(&self) -> bool {
        matches!(self.status, StepStatus::Unsaturated { .. })
    }

    pub fn is_saturating(&self) -> bool {
        matches!(self.status, StepStatus::Saturating { .. })
    }

    pub fn is_saturated(&self) -> bool {
        matches!(self.status, StepStatus::Saturated { .. })
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
