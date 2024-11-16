use std::{any::TypeId, hash::{Hash, Hasher}};

use super::{Transposer, TransposerInput};


pub unsafe trait HasErasedInput<T: Transposer> {
    fn get_input_type(&self) -> TypeId;

    fn get_input_type_value_hash(&self, state: &mut dyn Hasher);

    fn inputs_eq(&self, other: &dyn HasErasedInput<T>) -> bool;

    fn get_raw_input(&self) -> *const ();
}

/// used to implement HasErasedInput automatically much easier.
pub trait HasInput<T: Transposer> {
    type Input: TransposerInput<Base = T>;

    fn get_input(&self) -> &Self::Input;
}

unsafe impl<T: Transposer, U: HasInput<T>> HasErasedInput<T> for U {
    fn get_input_type(&self) -> TypeId {
        TypeId::of::<U::Input>()
    }

    fn get_input_type_value_hash(&self, mut state: &mut dyn Hasher) {
        self.get_input_type().hash(&mut state);
        self.get_input().hash(&mut state);
    }

    fn inputs_eq(&self, other: &dyn HasErasedInput<T>) -> bool {
        if self.get_input_type() != other.get_input_type() {
            return false;
        }

        let self_input = self.get_input();
        let other_input = unsafe { &*(other.get_raw_input() as *const U::Input) };

        self_input == other_input
    }

    fn get_raw_input(&self) -> *const () {
        self.get_input() as *const U::Input as *const ()
    }
}

impl<I: TransposerInput> HasInput<I::Base> for I {
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        self
    }
}

#[repr(transparent)]
pub struct ErasedInput<T: Transposer>(dyn HasErasedInput<T>);

impl<T: Transposer> Hash for ErasedInput<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.get_input_type_value_hash(state);
    }
}

impl<T: Transposer> PartialEq for ErasedInput<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.inputs_eq(&other.0)
    }
}

impl<T: Transposer> Eq for ErasedInput<T> {}

unsafe impl<T: Transposer> HasErasedInput<T> for ErasedInput<T> {
    fn get_input_type(&self) -> TypeId {
        self.0.get_input_type()
    }

    fn get_input_type_value_hash(&self, state: &mut dyn Hasher) {
        self.0.get_input_type_value_hash(state);
    }

    fn inputs_eq(&self, other: &dyn HasErasedInput<T>) -> bool {
        self.0.inputs_eq(other)
    }

    fn get_raw_input(&self) -> *const () {
        self.0.get_raw_input()
    }
}

impl<T: Transposer> From<&dyn HasErasedInput<T>> for &ErasedInput<T> {
    fn from(value: &dyn HasErasedInput<T>) -> Self {
        // SAFETY: this is safe because ErasedInput is a repr(transparent) around dyn HasErasedInput
        unsafe { core::mem::transmute(value) }
    }
}

impl<T: Transposer> From<&ErasedInput<T>> for &dyn HasErasedInput<T> {
    fn from(value: &ErasedInput<T>) -> Self {
        // SAFETY: this is safe because ErasedInput is a repr(transparent) around dyn HasErasedInput
        unsafe { core::mem::transmute(value) }
    }
}

impl<T: Transposer> From<Box<dyn HasErasedInput<T>>> for Box<ErasedInput<T>> {
    fn from(value: Box<dyn HasErasedInput<T>>) -> Self {
        // SAFETY: this is safe because ErasedInput is a repr(transparent) around dyn HasErasedInput
        unsafe { core::mem::transmute(value) }
    }
}

impl<T: Transposer> From<Box<ErasedInput<T>>> for Box<dyn HasErasedInput<T>> {
    fn from(value: Box<ErasedInput<T>>) -> Self {
        // SAFETY: this is safe because ErasedInput is a repr(transparent) around dyn HasErasedInput
        unsafe { core::mem::transmute(value) }
    }
}