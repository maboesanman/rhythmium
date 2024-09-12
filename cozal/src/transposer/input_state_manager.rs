use std::{
    any::TypeId,
    collections::HashMap,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll, Waker},
};

use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

pub struct InputStateManager<T> {
    request: Option<TypeId>,
    states: HashMap<TypeId, Box<dyn InputStateMapErased>>,
    phantom: PhantomData<T>,
}

struct InputStateMap<I: TransposerInput> {
    requested_input: Option<(I, Waker)>,
    states: HashMap<I, NonNull<I::InputState>>,
}

trait InputStateMapErased {
    // input is owned.
    fn set_request(&mut self, input: Option<NonNull<()>>);

    fn get_request_waker(&self) -> Option<&Waker>;

    // result is not owned.
    fn get_request(&self) -> Option<NonNull<()>>;

    // neither input nor result are owned pointers.
    fn get(&self, input: NonNull<()>) -> Option<NonNull<()>>;

    // neither input nor result are owned pointers.
    fn insert(&mut self, input: NonNull<()>, state: NonNull<()>) -> Option<NonNull<()>>;
}

impl<I: TransposerInput> InputStateMapErased for InputStateMap<I> {
    fn set_request(&mut self, input: Option<NonNull<()>>) {
        self.requested_input = input.map(|i| unsafe { i.cast().read() });
    }

    fn get_request_waker(&self) -> Option<&Waker> {
        self.requested_input.as_ref().map(|(_, waker)| waker)
    }

    fn get_request(&self) -> Option<NonNull<()>> {
        self.requested_input
            .as_ref()
            .map(|i| NonNull::from(i).cast())
    }

    fn get(&self, input: NonNull<()>) -> Option<NonNull<()>> {
        let input: I = unsafe { input.cast().read() };
        self.states.get(&input).map(|s| s.cast())
    }

    fn insert(&mut self, input: NonNull<()>, state: NonNull<()>) -> Option<NonNull<()>> {
        let input: I = unsafe { input.cast().read() };
        self.states.insert(input, state.cast()).map(|s| s.cast())
    }
}

impl<T> Default for InputStateManager<T> {
    fn default() -> Self {
        Self {
            request: None,
            states: HashMap::new(),
            phantom: PhantomData,
        }
    }
}

impl<T: Transposer> InputStateManager<T> {
    pub fn get_requested_input_type_id(&self) -> Option<TypeId> {
        self.request
    }

    pub fn get_requested_input<I>(&self) -> Option<I>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        unsafe {
            self.states
                .get(&TypeId::of::<I>())?
                .get_request()?
                .cast()
                .read()
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
        let outer_map_entry = self.states.entry(TypeId::of::<I>());

        let inner_map = outer_map_entry.or_insert_with(|| {
            Box::new(InputStateMap::<I> {
                requested_input: Some((input, waker)),
                states: HashMap::new(),
            })
        });

        let input_erased = NonNull::from(&input);

        if let Some(input_state_erased) = inner_map.get(input_erased.cast()) {
            return Some(input_state_erased.cast());
        }

        if self.request.is_some() {
            return None;
        }

        self.request = Some(TypeId::of::<I>());
        inner_map.set_request(Some(input_erased.cast()));

        None
    }

    pub fn provide_input_state<I>(
        &mut self,
        input: I,
        state: NonNull<I::InputState>,
    ) -> Result<(), NonNull<I::InputState>>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        match self.request {
            Some(type_id) => {
                if type_id != TypeId::of::<I>() {
                    return Err(state);
                }
            }
            None => return Err(state),
        }

        let inner = self.states.get_mut(&TypeId::of::<I>()).unwrap();

        if let Some(waker) = inner.get_request_waker() {
            waker.wake_by_ref();
        }

        inner.set_request(None);

        inner.insert(NonNull::from(&input).cast(), state.cast());

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
