use std::{
    borrow::Borrow, collections::HashSet, future::Future, marker::PhantomData, pin::Pin, ptr::NonNull, task::{Context, Poll, Waker}
};

use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

use super::input_erasure::{ErasedInput, ErasedInputState, HasErasedInput, HasInput};


pub struct InputStateManager<T: Transposer> {
    request: RequestStatus<T>,
    states: HashSet<Box<ErasedInputState<T>>>,
}

impl<T: Transposer> Default for InputStateManager<T> {
    fn default() -> Self {
        Self { request: Default::default(), states: Default::default() }
    }
}

#[derive(Default)]
enum RequestStatus<T: Transposer> {
    Requested(Waker, Box<ErasedInput<T>>),
    Accepted(Waker),

    #[default]
    None,
}

pub struct InputState<I: TransposerInput> {
    input: I,
    state: I::InputState,
}

impl<I: TransposerInput> HasInput<I::Base> for InputState<I> {
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        &self.input
    }
}

trait HasErasedInputState<T: Transposer>: HasErasedInput<T> {
    // this returns an &'_ I::InputState
    fn get_input_state(&self) -> NonNull<()>;
}

impl<I: TransposerInput> HasErasedInputState<I::Base> for InputState<I> {
    fn get_input_state(&self) -> NonNull<()> {
        NonNull::from(&self.state).cast()
    }
}

impl<T: Transposer> InputStateManager<T> {
    pub fn try_accept_request(&mut self) -> Option<Box<ErasedInput<T>>> {
        match core::mem::take(&mut self.request) {
            RequestStatus::Requested(waker, input) => {
                self.request = RequestStatus::Accepted(waker);
                Some(input)
            }
            RequestStatus::Accepted(_) => panic!("should't be attempting to accept while already accepted"),
            RequestStatus::None => None,
        }
    }

    pub fn get_or_request_state<I>(
        &mut self,
        input: I,
        waker: Waker,
    ) -> Option<NonNull<I::InputState>>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        match self.request {
            RequestStatus::None => {},
            _ => panic!("shouldn't be requesting while already requested"),
        }

        let query: &dyn HasErasedInput<T> = &input;
        let query: &ErasedInput<T> = query.into();

        if let Some(item) = self.states.get(query) {
            // SAFETY: we know that the item found must match the query type
            return Some(item.as_dyn().get_input_state().cast())
        }

        let boxed: Box<dyn HasErasedInput<T>> = Box::new(input);

        self.request = RequestStatus::Requested(waker, boxed.into());

        None
    }

    pub fn provide_input_state(
        &mut self,
        erased_state: Box<ErasedInputState<T>>
    ) -> Result<(), Box<ErasedInputState<T>>> {
        let waker = match &self.request {
            RequestStatus::Requested(..) => return Err(erased_state),
            RequestStatus::Accepted(waker) => waker,
            RequestStatus::None => return Err(erased_state),
        };

        let query: &ErasedInput<T> = erased_state.as_ref().borrow();

        if self.states.contains(query) {
            return Err(erased_state);
        }

        self.states.insert(erased_state);

        waker.wake_by_ref();

        self.request = RequestStatus::None;

        Ok(())
    }
}

pub struct GetInputStateFuture<'fut, 'update: 'fut, I: TransposerInput> {
    input_state_manager: NonNull<InputStateManager<I::Base>>,
    phantom_ism: PhantomData<&'fut mut InputStateManager<I::Base>>,
    phantom_update: PhantomData<fn() -> &'update I::InputState>,
    input: I,
    complete: bool,
}

impl<'fut, 'update: 'fut, I: TransposerInput> GetInputStateFuture<'fut, 'update, I> {
    pub fn new(input_state_manager: NonNull<InputStateManager<I::Base>>, input: I) -> Self {
        Self {
            input_state_manager,
            phantom_ism: PhantomData,
            phantom_update: PhantomData,
            input,
            complete: false,
        }
    }
}

impl<'fut, 'update: 'fut, I: TransposerInput> Future for GetInputStateFuture<'fut, 'update, I> {
    type Output = &'update I::InputState;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.complete {
            return Poll::Pending;
        }
        let input = self.input;
        let this = unsafe { self.get_unchecked_mut() };
        let input_state_manager = unsafe { this.input_state_manager.as_mut() };
        match input_state_manager.get_or_request_state(input, cx.waker().clone()) {
            Some(input_state) => {
                this.complete = true;
                #[allow(dropping_references)]
                drop(this);
                Poll::Ready(unsafe { input_state.as_ref() })
            }
            None => Poll::Pending,
        }
    }
}
