use std::ffi::c_void;

pub trait Anonymizable<const MUT: bool, A, R> {
    type Anonymized;

    fn anonymize(self) -> Anonymized<MUT, A, R, Self>;
}

pub struct Anonymized<const MUT: bool, A, R, F: Anonymizable<MUT, A, R> + ?Sized> {
    pub function: F::Anonymized,
    pub data: *mut c_void,
    pub drop: unsafe extern "C" fn(*mut c_void),
}

pub fn anonymize<A, R, F: Anonymizable<false, A, R>>(f: F) -> Anonymized<false, A, R, F> {
    f.anonymize()
}

pub fn anonymize_mut<A, R, F: Anonymizable<true, A, R>>(f: F) -> Anonymized<true, A, R, F> {
    f.anonymize()
}

macro_rules! impl_anonymizable_inner {
    ($mut:expr, $fn:ident, $($arg:ident),*) => {
        impl<$($arg,)* R, F: $fn($($arg),*) -> R> Anonymizable<$mut, ($($arg,)*), R> for F {
            type Anonymized = unsafe extern "C" fn(*mut c_void, $($arg,)*) -> R;

            fn anonymize(self) -> Anonymized<$mut, ($($arg,)*), R, Self> {
                let ptr = Box::into_raw(Box::new(self));
                #[allow(non_snake_case)]
                unsafe extern "C" fn anonymous<$($arg,)* R, F: $fn($($arg,)*) -> R>(data: *mut c_void, $($arg: $arg,)*) -> R {
                    (*data.cast::<F>())($($arg,)*)
                }
                unsafe extern "C" fn drop<$($arg,)* R, F: $fn($($arg,)*) -> R>(data: *mut c_void) {
                    data.cast::<F>().drop_in_place();
                }
                Anonymized {
                    function: anonymous::<$($arg,)* R, F>,
                    data: ptr.cast(),
                    drop: drop::<$($arg,)* R, F>,
                }
            }
        }
    };
}

macro_rules! impl_anonymizable {
    ($($arg:ident),*) => {
        impl_anonymizable_inner!(false, Fn, $($arg),*);
        impl_anonymizable_inner!(true, FnMut, $($arg),*);
    };
}

impl_anonymizable!();
impl_anonymizable!(A1);
impl_anonymizable!(A1, A2);
impl_anonymizable!(A1, A2, A3);
impl_anonymizable!(A1, A2, A3, A4);
impl_anonymizable!(A1, A2, A3, A4, A5);
impl_anonymizable!(A1, A2, A3, A4, A5, A6);
impl_anonymizable!(A1, A2, A3, A4, A5, A6, A7);
impl_anonymizable!(A1, A2, A3, A4, A5, A6, A7, A8);
