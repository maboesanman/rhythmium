use std::{any::TypeId, cmp::Ordering};

use archery::SharedPointerKind;

use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

use super::{input_sub_step::InputSubStep, BoxedSubStep, SubStep};

/// A single type-erased input.
#[repr(transparent)]
pub struct BoxedInput<'t, T: Transposer + 't, P: SharedPointerKind + 't>(BoxedSubStep<'t, T, P>);

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> TryFrom<BoxedSubStep<'t, T, P>>
    for BoxedInput<'t, T, P>
{
    type Error = BoxedInputConversionError;

    fn try_from(mut value: BoxedSubStep<'t, T, P>) -> Result<Self, Self::Error> {
        if value.as_ref().is_input() {
            value.as_mut().desaturate();
            Ok(Self(value))
        } else {
            Err(BoxedInputConversionError::NotInput)
        }
    }
}

#[derive(Debug)]
pub enum BoxedInputConversionError {
    NotInput,
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> From<BoxedInput<'t, T, P>>
    for BoxedSubStep<'t, T, P>
{
    fn from(value: BoxedInput<'t, T, P>) -> Self {
        value.0
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> std::fmt::Debug for BoxedInput<'t, T, P>
where
    T::Time: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedInputSubStep")
            .field("time", &self.get_time())
            .field("input_type_id", &self.get_input_type_id())
            .finish()
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> PartialOrd for BoxedInput<'t, T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> Ord for BoxedInput<'t, T, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> PartialEq for BoxedInput<'t, T, P> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> Eq for BoxedInput<'t, T, P> {}

impl<'t, T: Transposer + 't, P: SharedPointerKind + 't> BoxedInput<'t, T, P> {
    /// Create a new boxed input.
    pub fn new<I>(time: T::Time, input: I, input_event: I::InputEvent) -> Self
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I> + Clone,
    {
        InputSubStep::<T, P, I>::new_boxed(time, input, input_event)
            .try_into()
            .unwrap()
    }

    /// Get the time of the input.
    pub fn get_time(&self) -> T::Time {
        self.0.as_ref().get_time()
    }

    /// Get the type id of the input.
    pub fn get_input_type_id(&self) -> TypeId {
        self.0.as_ref().input_sort().unwrap().1
    }

    /// Try to get the input.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is not the correct type.
    pub fn get_input<I>(&self) -> Result<I, GetInputError>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I> + Clone,
    {
        if TypeId::of::<I>() != self.0.as_ref().input_sort().unwrap().1 {
            return Err(GetInputError::WrongType);
        }

        let input_ptr = self.0.as_ref() as *const dyn SubStep<T, P> as *const InputSubStep<T, P, I>;
        let input = unsafe { &*input_ptr };

        Ok(*input.get_input())
    }

    /// Check if the input is the same as the provided input.
    ///
    /// # Returns
    ///
    /// Returns true if the input is the same as the provided input.
    ///
    /// Returns false if the input is a different type, or if the input is not the same as the provided input.
    pub fn is_from_input<I>(&self, input: I) -> bool
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I> + Clone,
    {
        match self.get_input::<I>() {
            Ok(value) => value == input,
            Err(_) => false,
        }
    }
}

#[derive(Debug)]
pub enum GetInputError {
    WrongType,
}
