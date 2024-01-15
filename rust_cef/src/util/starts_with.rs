
/// # Safety
/// `T` must be the first field of `Self`, and use repr(C).
pub unsafe trait StartsWith<T> {}

pub trait StartsWithExt<T>: StartsWith<T> {
    fn get_start(&self) -> &T {
        let self_ptr = self as *const Self;
        let start_ptr = self_ptr.cast::<T>();
        unsafe { &*start_ptr }
    }

    fn get_start_mut(&mut self) -> &mut T {
        let self_ptr = self as *mut Self;
        let start_ptr = self_ptr.cast::<T>();
        unsafe { &mut *start_ptr }
    }
}

impl<T, U> StartsWithExt<T> for U where U: StartsWith<T> {}
