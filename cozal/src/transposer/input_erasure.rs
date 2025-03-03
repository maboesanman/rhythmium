use std::{any::TypeId, borrow::Borrow, hash::{Hash, Hasher}, ptr::NonNull};

use super::{Transposer, TransposerInput};

/// A trait that allows for type erased interaction with an input.
pub unsafe trait HasErasedInput<T: Transposer> {

    /// Get the type of the input that this type holds.
    fn get_input_type(&self) -> TypeId;

    /// Get the hash of the input type and value.
    /// 
    /// first hashes TypeId::of::<Self::Input>() and then hashes the input value.
    fn get_input_type_value_hash(&self, state: &mut dyn Hasher);

    /// Check if the input of this type is equal to the input of another type.
    ///
    /// This must check if the input type is the same and then compare the input values.
    fn inputs_eq(&self, other: &dyn HasErasedInput<T>) -> bool;

    /// Get the sort value of the input.
    fn input_sort(&self) -> u64;

    /// Compare the input values.
    fn inputs_cmp(&self, other: &dyn HasErasedInput<T>) -> std::cmp::Ordering;

    /// Get the raw pointer to the input.
    fn get_raw_input(&self) -> NonNull<()>;

    fn clone_input(&self) -> Box<ErasedInput<T>>;
}

pub trait HasErasedInputExt<T: Transposer>: HasErasedInput<T> {
    fn get_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.get_input_type_value_hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Transposer, U: HasErasedInput<T> + ?Sized> HasErasedInputExt<T> for U {}

/// used to implement HasErasedInput automatically much easier.
pub trait HasInput<T: Transposer> {
    /// The input type that this type holds.
    type Input: TransposerInput<Base = T>;

    /// Get the input that this type holds.
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
        let other_input = unsafe { (other.get_raw_input().cast::<U::Input>()).as_ref() };

        self_input == other_input
    }

    fn input_sort(&self) -> u64 {
        U::Input::SORT
    }

    fn inputs_cmp(&self, other: &dyn HasErasedInput<T>) -> std::cmp::Ordering {
        match U::Input::SORT.cmp(&other.input_sort()) {
            std::cmp::Ordering::Equal => {},
            other => return other,
        }

        if self.get_input_type() != other.get_input_type() {
            panic!("two different input types with the same sort value");
        }

        let self_input = self.get_input();
        let other_input = unsafe { (other.get_raw_input().cast::<U::Input>()).as_ref() };

        self_input.cmp(other_input)
    }

    fn get_raw_input(&self) -> NonNull<()> {
        NonNull::from(self.get_input()).cast()
    }

    fn clone_input(&self) -> Box<ErasedInput<T>> {
        ErasedInput::new(*self.get_input())
    }
}

impl<I: TransposerInput> HasInput<I::Base> for I {
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        self
    }
}

/// A trait that allows for type erased interaction with an input state.
pub unsafe trait HasErasedInputState<T: Transposer>: HasErasedInput<T> {
    /// Get the raw pointer to the input state.
    fn get_input_state(&self) -> NonNull<()>;
}

/// An unsized type that is intended to be used via Borrow<ErasedInput<T>>.
#[repr(transparent)]
pub struct ErasedInput<T: Transposer>(dyn HasErasedInput<T>);

struct InnerErasedInput<I: TransposerInput>(I);

impl<I: TransposerInput> HasInput<I::Base> for InnerErasedInput<I> {
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        &self.0
    }
}

impl<T: Transposer> ErasedInput<T>
{
    /// Create a new ErasedInput from a concrete TransposerInput.
    pub fn new<I: TransposerInput<Base = T>>(input: I) -> Box<Self> {
        let inner: Box<dyn HasErasedInput<T>> = Box::new(InnerErasedInput(input));
        inner.into()
    }

    // pub fn get_hash(&self) -> u64 {
    //     let mut hasher = std::collections::hash_map::DefaultHasher::new();
    //     self.hash(&mut hasher);
    //     hasher.finish()
    // }
}

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

impl<T: Transposer> Ord for ErasedInput<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.inputs_cmp(&other.0)
    }
}

impl<T: Transposer> PartialOrd for ErasedInput<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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

    fn input_sort(&self) -> u64 {
        self.0.input_sort()
    }
    
    fn inputs_cmp(&self, other: &dyn HasErasedInput<T>) -> std::cmp::Ordering {
        self.0.inputs_cmp(other)
    }

    fn get_raw_input(&self) -> NonNull<()> {
        self.0.get_raw_input()
    }

    fn clone_input(&self) -> Box<ErasedInput<T>> {
        self.0.clone_input()
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

/// An unsized type that is intended to be used via Borrow<ErasedInput<T>>.
#[repr(transparent)]
pub struct ErasedInputState<T: Transposer>(dyn HasErasedInputState<T>);

struct InnerErasedInputState<I: TransposerInput> {
    input: I,
    input_state: I::InputState,
}

impl<I: TransposerInput> HasInput<I::Base> for InnerErasedInputState<I> {
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        &self.input
    }
}

unsafe impl<I: TransposerInput> HasErasedInputState<I::Base> for InnerErasedInputState<I> {
    fn get_input_state(&self) -> NonNull<()> {
        NonNull::from(&self.input_state).cast()
    }
}

impl<T: Transposer> ErasedInputState<T> {
    /// Create a new ErasedInput from a concrete TransposerInput.
    pub fn new<I: TransposerInput<Base = T>>(input: I, input_state: I::InputState) -> Box<Self>{
        let inner = InnerErasedInputState {
            input,
            input_state,
        };
        let inner: Box<dyn HasErasedInputState<T>> = Box::new(inner);
        inner.into()
    }

    /// Get a dyn reference to the inner trait object.
    pub fn as_dyn(&self) -> &dyn HasErasedInputState<T> {
        &self.0
    }
}

impl<T: Transposer> Hash for ErasedInputState<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.get_input_type_value_hash(state);
    }
}

impl<T: Transposer> PartialEq for ErasedInputState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.inputs_eq(&other.0)
    }
}

impl<T: Transposer> Eq for ErasedInputState<T> {}

unsafe impl<T: Transposer> HasErasedInput<T> for ErasedInputState<T> {
    fn get_input_type(&self) -> TypeId {
        self.0.get_input_type()
    }

    fn get_input_type_value_hash(&self, state: &mut dyn Hasher) {
        self.0.get_input_type_value_hash(state);
    }

    fn inputs_eq(&self, other: &dyn HasErasedInput<T>) -> bool {
        self.0.inputs_eq(other)
    }

    fn input_sort(&self) -> u64 {
        self.0.input_sort()
    }
    
    fn inputs_cmp(&self, other: &dyn HasErasedInput<T>) -> std::cmp::Ordering {
        self.0.inputs_cmp(other)
    }

    fn get_raw_input(&self) -> NonNull<()> {
        self.0.get_raw_input()
    }
    
    fn clone_input(&self) -> Box<ErasedInput<T>> {
        self.0.clone_input()
    }
}

impl<T: Transposer> From<&dyn HasErasedInputState<T>> for &ErasedInputState<T> {
    fn from(value: &dyn HasErasedInputState<T>) -> Self {
        // SAFETY: this is safe because ErasedInputState is a repr(transparent) around dyn HasErasedInputState
        unsafe { core::mem::transmute(value) }
    }
}

impl<T: Transposer> From<&ErasedInputState<T>> for &dyn HasErasedInputState<T> {
    fn from(value: &ErasedInputState<T>) -> Self {
        // SAFETY: this is safe because ErasedInputState is a repr(transparent) around dyn HasErasedInputState
        unsafe { core::mem::transmute(value) }
    }
}

impl<T: Transposer> From<Box<dyn HasErasedInputState<T>>> for Box<ErasedInputState<T>> {
    fn from(value: Box<dyn HasErasedInputState<T>>) -> Self {
        // SAFETY: this is safe because ErasedInputState is a repr(transparent) around dyn HasErasedInputState
        unsafe { core::mem::transmute(value) }
    }
}

impl<T: Transposer> From<Box<ErasedInputState<T>>> for Box<dyn HasErasedInputState<T>> {
    fn from(value: Box<ErasedInputState<T>>) -> Self {
        // SAFETY: this is safe because ErasedInputState is a repr(transparent) around dyn HasErasedInputState
        unsafe { core::mem::transmute(value) }
    }
}
unsafe impl<T: Transposer> HasErasedInputState<T> for ErasedInputState<T> {
    fn get_input_state(&self) -> NonNull<()> {
        self.0.get_input_state()
    }
}

impl<T: Transposer> Borrow<ErasedInput<T>> for ErasedInputState<T> {
    fn borrow(&self) -> &ErasedInput<T> {
        let inner: &dyn HasErasedInputState<T> = &self.0;
        let casted_inner: &dyn HasErasedInput<T> = inner;
        casted_inner.into()
    }
}

impl<T: Transposer> Borrow<ErasedInput<T>> for Box<ErasedInputState<T>> {
    fn borrow(&self) -> &ErasedInput<T> {
        let inner: &dyn HasErasedInputState<T> = &self.0;
        let casted_inner: &dyn HasErasedInput<T> = inner;
        casted_inner.into()
    }
}
