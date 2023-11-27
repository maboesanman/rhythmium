use std::mem::ManuallyDrop;

use cef_sys::{cef_string_userfree_t, cef_string_utf16_t};

const HEADER_BYTES: usize = std::mem::size_of::<usize>() >> 1;
const HEADER_LENGTH: usize = HEADER_BYTES >> 1;

#[repr(C)]
struct CefStr {
    // this is the length of the data, not the length of the header.
    length: usize,
    data: [u16],
}

impl<T: IntoIterator<Item = u16>> From<T> for Box<CefStr> {
    fn from(value: T) -> Self {
        let header_data = (0..HEADER_LENGTH).map(|_| 0);
        let all_data = Box::into_raw(header_data.chain(value).collect());
        let (start_ptr, length) = all_data.to_raw_parts();

        unsafe { *start_ptr.cast::<usize>() = length - HEADER_LENGTH };

        let cef_str_ptr = std::ptr::from_raw_parts_mut::<CefStr>(start_ptr, length);

        unsafe { Box::from_raw(cef_str_ptr) }
    }
}

impl From<Box<CefStr>> for cef_string_utf16_t {
    fn from(val: Box<CefStr>) -> Self {
        let length = val.length;

        let mut manually_drop = ManuallyDrop::new(val);

        let data_ptr = manually_drop.data.as_mut_ptr();

        cef_string_utf16_t {
            str_: data_ptr,
            length,
            dtor: Some(CefStr::drop_string),
        }
    }
}

impl CefStr {
    unsafe extern "C" fn drop_string(ptr: *mut u16) {
        let ptr = CefStr::from_data_raw(ptr);
        let _ = Box::from_raw(ptr);
    }

    unsafe fn from_data_raw(data: *mut u16) -> *mut Self {
        let start_ptr = unsafe { data.byte_sub(HEADER_BYTES) };
        let length = unsafe { *start_ptr.cast::<usize>() };

        std::ptr::from_raw_parts_mut::<Self>(start_ptr.cast(), length + HEADER_LENGTH)
    }
}

pub fn str_into_cef_string_utf16(string: &str) -> cef_string_utf16_t {
    let string = string.encode_utf16();
    let string: Box<CefStr> = string.into();
    string.into()
}

/// # Safety
///
/// `cef_string` must be a valid pointer, and must not be dropped by anything else.
pub unsafe fn cef_string_userfree_into_string(cef_string: cef_string_userfree_t) -> Option<String> {
    let boxed = unsafe { Box::from_raw(cef_string) };
    if boxed.str_.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(boxed.str_, boxed.length) };

    let value = String::from_utf16_lossy(bytes);

    unsafe { boxed.dtor.unwrap()(boxed.str_) };

    Some(value)
}

/// # Safety
///
/// `cef_string` must be a valid pointer to a `cef_string_userfree_t`, and must live until the end of this function.
pub unsafe fn cef_string_utf16_into_string(
    cef_string: *const cef_string_utf16_t,
) -> Option<String> {
    let cef_string = unsafe { cef_string.as_ref() }.unwrap();
    let bytes = unsafe { std::slice::from_raw_parts(cef_string.str_, cef_string.length) };

    let value = String::from_utf16_lossy(bytes);

    Some(value)
}
