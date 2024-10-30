use std::{
    any::{self, TypeId}, hash::{Hash, Hasher}, ptr::{self, DynMetadata}
};

use crate::{
    source::Source,
    transposer::{Transposer, TransposerInput, TransposerInputEventHandler},
};

unsafe trait ErasedInputSourceTrait<T: Transposer>{
    fn get_input_type(&self) -> TypeId;

    fn get_input_type_value_hash(&self, state: &mut dyn Hasher);

    fn inputs_eq(&self, other: &dyn ErasedInputSourceTrait<T>) -> bool;

    fn get_raw_input(&self) -> *const ();

    fn get_raw_src_data(&self) -> *const ();

    fn get_raw_src_data_mut(&mut self) -> *mut ();

    fn get_raw_src_metadata(&self) -> *const ();
}

trait ErasedInputSourceTraitExt<T: Transposer>: ErasedInputSourceTrait<T> {
    fn get_input<I>(&self) -> Option<I>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        if self.get_input_type() != any::TypeId::of::<I>() {
            return None;
        }

        unsafe { Some(*(self.get_raw_input() as *const I)) }
    }

    fn get<I>(
        &self,
    ) -> Option<
        &dyn Source<
            Time = <I::Base as Transposer>::Time,
            Event = I::InputEvent,
            State = I::InputState,
        >,
    >
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        if self.get_input_type() != any::TypeId::of::<I>() {
            return None;
        }

        let data_ptr = self.get_raw_src_data();
        let metadata_ptr = self.get_raw_src_metadata();

        let metadata_ptr = metadata_ptr
            as *const DynMetadata<
                dyn Source<
                    Time = <I::Base as Transposer>::Time,
                    Event = I::InputEvent,
                    State = I::InputState,
                >,
            >;

        unsafe {
            let dyn_source = ptr::from_raw_parts(data_ptr, *metadata_ptr);

            Some(&*dyn_source)
        }
    }

    fn get_mut<I>(
        &mut self,
    ) -> Option<
        &mut dyn Source<
            Time = <I::Base as Transposer>::Time,
            Event = I::InputEvent,
            State = I::InputState,
        >,
    >
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        if self.get_input_type() != any::TypeId::of::<I>() {
            return None;
        }

        let data_ptr = self.get_raw_src_data_mut();
        let metadata_ptr = self.get_raw_src_metadata();

        let metadata_ptr = metadata_ptr
            as *const DynMetadata<
                dyn Source<
                    Time = <I::Base as Transposer>::Time,
                    Event = I::InputEvent,
                    State = I::InputState,
                >,
            >;

        unsafe {
            let dyn_source = ptr::from_raw_parts_mut(data_ptr, *metadata_ptr);

            Some(&mut *dyn_source)
        }
    }
}

impl<T: Transposer, U: ErasedInputSourceTrait<T> + ?Sized> ErasedInputSourceTraitExt<T> for U {}

struct ErasedInputSourceTraitImpl<I: TransposerInput, Src> {
    input: I,
    source: Src,
    source_metadata: DynMetadata<
        dyn Source<
            Time = <I::Base as Transposer>::Time,
            Event = I::InputEvent,
            State = I::InputState,
        >,
    >,
}

impl<I: TransposerInput, Src> ErasedInputSourceTraitImpl<I, Src>
where
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
{
    fn new(input: I, source: Src) -> Self {
        Self {
            input,
            source_metadata: ptr::metadata(
                &source as *const Src
                    as *const dyn Source<
                        Time = <I::Base as Transposer>::Time,
                        Event = I::InputEvent,
                        State = I::InputState,
                    >,
            ),
            source,
        }
    }
}

unsafe impl<I: TransposerInput, Src> ErasedInputSourceTrait<I::Base>
    for ErasedInputSourceTraitImpl<I, Src>
where
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
{
    fn get_input_type(&self) -> TypeId {
        any::TypeId::of::<I>()
    }
    
    fn get_input_type_value_hash(&self, mut state: &mut dyn Hasher) {
        self.input.hash(&mut state);
    }

    fn inputs_eq(&self, other: &dyn ErasedInputSourceTrait<I::Base>) -> bool {
        if self.get_input_type() != other.get_input_type() {
            return false;
        }

        unsafe {
            let self_input = &self.input;
            let other_input = &*(other.get_raw_input() as *const I);

            self_input == other_input
        }
    }

    fn get_raw_input(&self) -> *const () {
        &self.input as *const I as *const ()
    }

    fn get_raw_src_data(&self) -> *const () {
        &self.source as *const Src as *const ()
    }

    fn get_raw_src_data_mut(&mut self) -> *mut () {
        &mut self.source as *mut Src as *mut ()
    }

    fn get_raw_src_metadata(&self) -> *const () {
        &self.source_metadata as *const DynMetadata<_> as *const ()
    }
}

pub struct ErasedInputSource<'src, T: Transposer> {
    inner: Box<dyn 'src + ErasedInputSourceTrait<T>>,
}

#[allow(dead_code)]
impl<'src, T: Transposer> ErasedInputSource<'src, T> {
    pub fn new<I, Src>(input: I, source: Src) -> Self
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
        Src: 'src + Source<
            Time = T::Time,
            Event = I::InputEvent,
            State = I::InputState
        >,
    {
        Self {
            inner: Box::new(ErasedInputSourceTraitImpl::new(input, source)),
        }
    }

    pub fn get_input_type(&self) -> TypeId {
        self.inner.get_input_type()
    }

    pub fn get_input<I>(&self) -> Option<I>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        self.inner.get_input::<I>()
    }

    pub fn get<I>(
        &self,
    ) -> Option<
        &dyn Source<
            Time = <I::Base as Transposer>::Time,
            Event = I::InputEvent,
            State = I::InputState,
        >,
    >
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        self.inner.get::<I>()
    }

    pub fn get_mut<I>(
        &mut self,
    ) -> Option<
        &mut dyn Source<
            Time = <I::Base as Transposer>::Time,
            Event = I::InputEvent,
            State = I::InputState,
        >,
    >
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        self.inner.get_mut::<I>()
    }

    pub fn get_hash_for_input<I, H: Hasher>(input: I, state: &mut H) 
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        let input_type = any::TypeId::of::<I>();
        input_type.hash(state);
        input.hash(state);
    }
}

impl<T: Transposer> std::hash::Hash for ErasedInputSource<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.get_input_type().hash(state);
        self.inner.get_input_type_value_hash(state);
    }
}

impl<T: Transposer> PartialEq for ErasedInputSource<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.get_input_type() == other.inner.get_input_type()
        && self.inner.inputs_eq(other.inner.as_ref())
    }
}

impl<T: Transposer> Eq for ErasedInputSource<'_, T> {}