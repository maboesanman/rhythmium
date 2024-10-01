use std::{any::TypeId, cmp::Ordering};

use crate::transposer::{Transposer, TransposerInput, TransposerInputEventHandler};

trait InputRegistration<T: Transposer> {
    fn input_sort(&self) -> (u64, TypeId);
    fn register_input(&self, transposer: &mut T) -> bool;
    fn dyn_cmp(&self, other: &dyn InputRegistration<T>) -> Ordering;
}

struct SpecificInputRegistration<I> {
    input: I,
}

impl<T, I> InputRegistration<T> for SpecificInputRegistration<I>
where
    T: Transposer + TransposerInputEventHandler<I>,
    I: TransposerInput<Base = T>,
{
    fn input_sort(&self) -> (u64, TypeId) {
        (I::SORT, TypeId::of::<I>())
    }

    fn register_input(&self, transposer: &mut T) -> bool {
        transposer.register_input(self.input)
    }

    fn dyn_cmp(&self, other: &dyn InputRegistration<T>) -> Ordering {
        match self.input_sort().cmp(&other.input_sort()) {
            Ordering::Equal => {
                // we compared both the input sort and the input type_id in this comparison, so
                // now we can be sure the type of other is Self.
            }
            ne => return ne,
        }

        let other_ptr = other as *const dyn InputRegistration<T> as *const Self;
        let other = unsafe { &*other_ptr };

        self.input.cmp(&other.input)
    }
}

struct DynInputRegistration<T: Transposer> {
    register_input: Box<dyn InputRegistration<T>>,
}

impl<T: Transposer> DynInputRegistration<T> {
    fn new<I>(input: I) -> Self
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        Self {
            register_input: Box::new(SpecificInputRegistration { input }),
        }
    }

    fn register_input(&self, transposer: &mut T) -> bool {
        self.register_input.register_input(transposer)
    }
}

impl<T: Transposer> Ord for DynInputRegistration<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.register_input.dyn_cmp(other.register_input.as_ref())
    }
}

impl<T: Transposer> PartialOrd for DynInputRegistration<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Transposer> PartialEq for DynInputRegistration<T> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T: Transposer> Eq for DynInputRegistration<T> {}

pub struct PreInitStep<T: Transposer> {
    registrations: Vec<DynInputRegistration<T>>,
}

impl<T: Transposer> Default for PreInitStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Transposer> PreInitStep<T> {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
        }
    }

    pub fn add_input<I>(&mut self, input: I)
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
    {
        self.registrations.push(DynInputRegistration::new(input));
    }

    pub fn execute(mut self, mut transposer: T) -> Result<T, T> {
        self.registrations.sort();

        for registration in self.registrations {
            if !registration.register_input(&mut transposer) {
                return Err(transposer);
            }
        }

        if !transposer.prepare_to_init() {
            return Err(transposer);
        }

        Ok(transposer)
    }
}
