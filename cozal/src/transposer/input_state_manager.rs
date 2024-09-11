use std::{any::TypeId, collections::HashMap, marker::PhantomData, ptr::NonNull};

use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

pub struct InputStateManager<T> {
    request: Option<TypeId>,
    states: HashMap<TypeId, Box<dyn InputStateMapErased>>,
    phantom: PhantomData<T>,
}

struct InputStateMap<I: TransposerInput> {
    requested_input: Option<I>,
    states: HashMap<I, NonNull<I::InputState>>,
}

trait InputStateMapErased {
    // input is owned.
    fn set_request(&mut self, input: Option<NonNull<()>>);

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

    pub fn get_or_request_state<I>(&mut self, input: I) -> Option<NonNull<I::InputState>>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        let outer_map_entry = self.states.entry(TypeId::of::<I>());

        let inner_map = outer_map_entry.or_insert_with(|| {
            Box::new(InputStateMap::<I> {
                requested_input: Some(input),
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

        inner.set_request(None);

        inner.insert(NonNull::from(&input).cast(), state.cast());

        Ok(())
    }
}
