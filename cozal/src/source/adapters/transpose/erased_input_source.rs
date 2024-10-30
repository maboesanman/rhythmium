use std::{
    any::{self, Any, TypeId},
    borrow::Borrow,
    hash::{Hash, Hasher},
    ptr::{self, metadata, DynMetadata},
};

use crate::{
    source::Source,
    transposer::{Transposer, TransposerInput, TransposerInputEventHandler},
};

unsafe trait ErasedInputTrait<T: Transposer> {
    fn get_input_type(&self) -> TypeId;

    fn get_input_type_value_hash(&self, state: &mut dyn Hasher);

    fn inputs_eq(&self, other: &dyn ErasedInputTrait<T>) -> bool;

    fn get_raw_input(&self) -> *const ();
}

unsafe impl<I: TransposerInput> ErasedInputTrait<I::Base> for I {
    fn get_input_type(&self) -> TypeId {
        any::TypeId::of::<I>()
    }

    fn get_input_type_value_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }

    fn inputs_eq(&self, other: &dyn ErasedInputTrait<I::Base>) -> bool {
        if self.get_input_type() != other.get_input_type() {
            return false;
        }

        unsafe {
            let other_input = &*(other.get_raw_input() as *const I);

            self == other_input
        }
    }

    fn get_raw_input(&self) -> *const () {
        self as *const I as *const ()
    }
}

unsafe trait ErasedInputSourceTrait<T: Transposer>: ErasedInputTrait<T> {
    fn get_raw_src_data(&self) -> *const ();

    fn get_raw_src_data_mut(&mut self) -> *mut ();

    // SAFETY: this is actually a DynMetadata<dyn Source<...>>, but we can't express that here
    // since we don't have the input type available.
    // this must be transmuted before use.
    unsafe fn get_raw_src_metadata(&self) -> DynMetadata<dyn Any>;
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
        let metadata = unsafe { self.get_raw_src_metadata() };

        unsafe {
            let metadata = core::mem::transmute::<
                DynMetadata<dyn Any>,
                DynMetadata<
                    dyn Source<
                        Time = <I::Base as Transposer>::Time,
                        Event = I::InputEvent,
                        State = I::InputState,
                    >,
                >,
            >(metadata);

            let dyn_source = ptr::from_raw_parts(data_ptr, metadata);

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
        let metadata = unsafe { self.get_raw_src_metadata() };

        unsafe {
            let metadata = core::mem::transmute::<
                DynMetadata<dyn Any>,
                DynMetadata<
                    dyn Source<
                        Time = <I::Base as Transposer>::Time,
                        Event = I::InputEvent,
                        State = I::InputState,
                    >,
                >,
            >(metadata);
            let dyn_source = ptr::from_raw_parts_mut(data_ptr, metadata);

            Some(&mut *dyn_source)
        }
    }
}

impl<T: Transposer, U: ErasedInputSourceTrait<T> + ?Sized> ErasedInputSourceTraitExt<T> for U {}

struct ErasedInputSourceTraitImpl<I: TransposerInput, Src> {
    input: I,
    source: Src,
    // source_metadata: DynMetadata<
    //     dyn Source<
    //         Time = <I::Base as Transposer>::Time,
    //         Event = I::InputEvent,
    //         State = I::InputState,
    //     >,
    // >,
}

impl<I: TransposerInput, Src> ErasedInputSourceTraitImpl<I, Src>
where
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
{
    fn new(input: I, source: Src) -> Self {
        Self { input, source }
    }
}

unsafe impl<I: TransposerInput, Src> ErasedInputTrait<I::Base>
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

    fn inputs_eq(&self, other: &dyn ErasedInputTrait<I::Base>) -> bool {
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
}

unsafe impl<I: TransposerInput, Src> ErasedInputSourceTrait<I::Base>
    for ErasedInputSourceTraitImpl<I, Src>
where
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
{
    fn get_raw_src_data(&self) -> *const () {
        &self.source as *const Src as *const ()
    }

    fn get_raw_src_data_mut(&mut self) -> *mut () {
        &mut self.source as *mut Src as *mut ()
    }

    unsafe fn get_raw_src_metadata(&self) -> DynMetadata<dyn Any> {
        let src_ptr = &self.source as *const Src
            as *const dyn Source<
                Time = <I::Base as Transposer>::Time,
                Event = I::InputEvent,
                State = I::InputState,
            >;
        let metadata = metadata(src_ptr);

        unsafe { core::mem::transmute(metadata) }
    }
}

#[repr(transparent)]
pub struct ErasedInputSource<'src, T: Transposer> {
    inner: Box<dyn 'src + ErasedInputSourceTrait<T>>,
}

#[allow(dead_code)]
impl<'src, T: Transposer> ErasedInputSource<'src, T> {
    pub fn new<I, Src>(input: I, source: Src) -> Self
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
        Src: 'src + Source<Time = T::Time, Event = I::InputEvent, State = I::InputState>,
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

#[repr(transparent)]
pub struct ErasedInputQuery<T: Transposer> {
    inner: dyn ErasedInputTrait<T>,
}

impl<T: Transposer> ErasedInputQuery<T> {
    pub fn new<I: TransposerInput<Base = T>>(input: &I) -> &Self {
        let inner: &dyn ErasedInputTrait<T> = input;
        let inner_ptr: *const dyn ErasedInputTrait<T> = inner;
        let outer_ptr: *const ErasedInputQuery<T> = inner_ptr as *const ErasedInputQuery<T>;
        unsafe { &*outer_ptr }
    }
}

impl<T: Transposer> std::hash::Hash for ErasedInputQuery<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.get_input_type().hash(state);
        self.inner.get_input_type_value_hash(state);
    }
}

impl<T: Transposer> PartialEq for ErasedInputQuery<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner.get_input_type() == other.inner.get_input_type()
            && self.inner.inputs_eq(&other.inner)
    }
}

impl<T: Transposer> Eq for ErasedInputQuery<T> {}

impl<'src, T: Transposer> Borrow<ErasedInputQuery<T>> for ErasedInputSource<'src, T> {
    fn borrow(&self) -> &ErasedInputQuery<T> {
        let inner_ref: &(dyn 'src + ErasedInputSourceTrait<T>) = self.inner.as_ref();
        let inner_ref: &dyn ErasedInputTrait<T> = inner_ref;
        let inner_ptr: *const dyn ErasedInputTrait<T> = inner_ref;
        let outer_ptr: *const ErasedInputQuery<T> = inner_ptr as *const ErasedInputQuery<T>;
        unsafe { &*outer_ptr }
    }
}
