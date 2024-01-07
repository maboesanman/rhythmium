use std::ffi::c_void;

pub trait Anonymizable<A, R> {
    type Anonymized;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void);
}

pub fn anonymize<A, R, F: Anonymizable<A, R>>(f: F) -> (F::Anonymized, *mut c_void) {
    f.anonymize()
}

// todo: make these macros

impl<R, F: Fn() -> R> Anonymizable<(), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<R, F: Fn() -> R>(data: *mut c_void) -> R {
            (*data.cast::<F>())()
        }
        (anonymous::<R, F>, ptr.cast())
    }
}

impl<A1, R, F: Fn(A1) -> R> Anonymizable<(A1,), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, R, F: Fn(A1) -> R>(data: *mut c_void, a1: A1) -> R {
            (*data.cast::<F>())(a1)
        }
        (anonymous::<A1, R, F>, ptr.cast())
    }
}

impl<A1, A2, R, F: Fn(A1, A2) -> R> Anonymizable<(A1, A2), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, R, F: Fn(A1, A2) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
        ) -> R {
            (*data.cast::<F>())(a1, a2)
        }
        (anonymous::<A1, A2, R, F>, ptr.cast())
    }
}

impl<A1, A2, A3, R, F: Fn(A1, A2, A3) -> R> Anonymizable<(A1, A2, A3), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2, A3) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, A3, R, F: Fn(A1, A2, A3) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
            a3: A3,
        ) -> R {
            (*data.cast::<F>())(a1, a2, a3)
        }
        (anonymous::<A1, A2, A3, R, F>, ptr.cast())
    }
}

impl<A1, A2, A3, A4, R, F: Fn(A1, A2, A3, A4) -> R> Anonymizable<(A1, A2, A3, A4), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2, A3, A4) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, A3, A4, R, F: Fn(A1, A2, A3, A4) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
            a3: A3,
            a4: A4,
        ) -> R {
            (*data.cast::<F>())(a1, a2, a3, a4)
        }
        (anonymous::<A1, A2, A3, A4, R, F>, ptr.cast())
    }
}

impl<A1, A2, A3, A4, A5, R, F: Fn(A1, A2, A3, A4, A5) -> R> Anonymizable<(A1, A2, A3, A4, A5), R>
    for F
{
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2, A3, A4, A5) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, A3, A4, A5, R, F: Fn(A1, A2, A3, A4, A5) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
            a3: A3,
            a4: A4,
            a5: A5,
        ) -> R {
            (*data.cast::<F>())(a1, a2, a3, a4, a5)
        }
        (anonymous::<A1, A2, A3, A4, A5, R, F>, ptr.cast())
    }
}

pub trait AnonymizableMut<A, R> {
    type Anonymized;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void);
}

pub fn anonymize_mut<A, R, F: AnonymizableMut<A, R>>(f: F) -> (F::Anonymized, *mut c_void) {
    f.anonymize()
}

// todo: make these macros

impl<R, F: FnMut() -> R> AnonymizableMut<(), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<R, F: FnMut() -> R>(data: *mut c_void) -> R {
            (*data.cast::<F>())()
        }
        (anonymous::<R, F>, ptr.cast())
    }
}

impl<A1, R, F: FnMut(A1) -> R> AnonymizableMut<(A1,), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, R, F: FnMut(A1) -> R>(data: *mut c_void, a1: A1) -> R {
            (*data.cast::<F>())(a1)
        }
        (anonymous::<A1, R, F>, ptr.cast())
    }
}

impl<A1, A2, R, F: FnMut(A1, A2) -> R> AnonymizableMut<(A1, A2), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, R, F: FnMut(A1, A2) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
        ) -> R {
            (*data.cast::<F>())(a1, a2)
        }
        (anonymous::<A1, A2, R, F>, ptr.cast())
    }
}

impl<A1, A2, A3, R, F: FnMut(A1, A2, A3) -> R> AnonymizableMut<(A1, A2, A3), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2, A3) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, A3, R, F: FnMut(A1, A2, A3) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
            a3: A3,
        ) -> R {
            (*data.cast::<F>())(a1, a2, a3)
        }
        (anonymous::<A1, A2, A3, R, F>, ptr.cast())
    }
}

impl<A1, A2, A3, A4, R, F: FnMut(A1, A2, A3, A4) -> R> AnonymizableMut<(A1, A2, A3, A4), R> for F {
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2, A3, A4) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, A3, A4, R, F: FnMut(A1, A2, A3, A4) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
            a3: A3,
            a4: A4,
        ) -> R {
            (*data.cast::<F>())(a1, a2, a3, a4)
        }
        (anonymous::<A1, A2, A3, A4, R, F>, ptr.cast())
    }
}

impl<A1, A2, A3, A4, A5, R, F: FnMut(A1, A2, A3, A4, A5) -> R>
    AnonymizableMut<(A1, A2, A3, A4, A5), R> for F
{
    type Anonymized = unsafe extern "C" fn(*mut c_void, A1, A2, A3, A4, A5) -> R;

    fn anonymize(self) -> (Self::Anonymized, *mut c_void) {
        let ptr = Box::into_raw(Box::new(self));
        unsafe extern "C" fn anonymous<A1, A2, A3, A4, A5, R, F: FnMut(A1, A2, A3, A4, A5) -> R>(
            data: *mut c_void,
            a1: A1,
            a2: A2,
            a3: A3,
            a4: A4,
            a5: A5,
        ) -> R {
            (*data.cast::<F>())(a1, a2, a3, a4, a5)
        }
        (anonymous::<A1, A2, A3, A4, A5, R, F>, ptr.cast())
    }
}
