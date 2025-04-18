use std::hint::unreachable_unchecked;
use std::ptr::NonNull;

pub fn get_occupied<T>(
    this: &mut Option<T>,
) -> Result<OccupiedExtEntry<'_, T>, VacantExtEntry<'_, T>> {
    let option: NonNull<Option<T>> = this.into();

    match this {
        Some(inner) => Ok(OccupiedExtEntry { option, inner }),
        None => Err(VacantExtEntry { option: this }),
    }
}

#[derive(Debug)]
pub struct OccupiedExtEntry<'a, T> {
    option: NonNull<Option<T>>,
    inner: &'a mut T,
}

impl<'a, T> OccupiedExtEntry<'a, T> {
    pub fn get_value(&self) -> &T {
        self.inner
    }

    pub fn get_value_mut(&mut self) -> &mut T {
        self.inner
    }

    pub fn into_value_mut(self) -> &'a mut T {
        self.inner
    }

    pub fn vacate(self) -> (VacantExtEntry<'a, T>, T) {
        let Self {
            inner: _,
            mut option,
        } = self;
        let option = unsafe { option.as_mut() };
        let value = option.take().unwrap();
        (VacantExtEntry { option }, value)
    }

    pub fn into_collection_mut(self) -> &'a mut Option<T> {
        let Self {
            inner: _,
            mut option,
        } = self;

        // SAFETY: this is kept alive by the lifetime 'a,
        // and does not alias entry because it's dropped.
        unsafe { option.as_mut() }
    }
}

#[derive(Debug)]
pub struct VacantExtEntry<'a, T> {
    option: &'a mut Option<T>,
}

impl<'a, T> VacantExtEntry<'a, T> {
    pub fn occupy(self, value: T) -> OccupiedExtEntry<'a, T> {
        let old = self.option.replace(value);
        match old {
            None => {}
            Some(_) => unsafe { unreachable_unchecked() },
        }
        let option: NonNull<Option<T>> = self.option.into();
        let inner = match self.option {
            Some(t) => t,
            None => unsafe { unreachable_unchecked() },
        };
        OccupiedExtEntry { option, inner }
    }

    pub fn into_collection_mut(self) -> &'a mut Option<T> {
        self.option
    }
}
