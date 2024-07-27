use std::{
    iter,
    mem::{transmute, ManuallyDrop},
    usize,
};

use cef_wrapper::cef_capi_sys::{cef_string_userfree_t, cef_string_utf16_t};

const HEADER_BYTES: usize = std::mem::size_of::<usize>();
const HEADER_LENGTH: usize = HEADER_BYTES >> 1;

/// The internal representation of a rust-originating cef_string_utf16_t.
///
/// This is a hack, because the destructor we pass to cef_string_utf16_t takes only a pointer to the data,
/// and therefore needs to store the length of data somewhere recoverable by the destructor.
/// we store it in the 8 bytes preceding the data, and set the str field of the cef_string_utf16_t to point to the data.
///
/// the destructor then recovers the length by subtracting HEADER_LENGTH from the pointer it is given,
/// then casts to a Box<CefStr> and drops it.
#[repr(C)]
struct CefStr {
    // this is the length of the data, not the length of the header.
    // the number of utf16 u16s present in data.
    // the actual number of u16s is slightly higher than this due to padding.
    length: usize,

    // this is the unsized u16 slice. because this is unsized, it must be the last field.
    data: [u16],
}

impl<T: IntoIterator<Item = u16>> From<T> for Box<CefStr> {
    fn from(value: T) -> Self {
        let header_data = iter::once(0usize);
        let body_data = U16AsUSizeIter::new(value.into_iter());

        let all_data: Box<[usize]> = header_data.chain(body_data).collect();

        let last_items = if all_data.len() == 1 {
            None
        } else {
            all_data
                .last()
                .map(|x: &usize| -> &[u16; HEADER_LENGTH] { unsafe { transmute(x) } })
        };

        let trailing_zeroes: usize = match last_items {
            None => 0,
            Some(array) => {
                let mut i = 0;
                loop {
                    if array[HEADER_LENGTH - 1 - i] != 0 {
                        break i;
                    }
                    i += 1;
                }
            }
        };

        let (start_ptr, length) = Box::into_raw(all_data).to_raw_parts();

        let u16_buffer_length = (length - 1) * HEADER_LENGTH;

        // we store the number of utf16 u16s in the data in the header.
        unsafe { *start_ptr.cast::<usize>() = (length - 1) * HEADER_LENGTH - trailing_zeroes };

        // the pointee metadata for *mut CefStr is the number of u16s in the slice.
        let cef_str_ptr = std::ptr::from_raw_parts_mut::<CefStr>(start_ptr, u16_buffer_length);

        unsafe { Box::from_raw(cef_str_ptr) }
    }
}

struct U16AsUSizeIter<I> {
    iter: I,
    exhausted: bool,
}

impl<I> U16AsUSizeIter<I> {
    fn new(iter: I) -> Self {
        Self {
            iter,
            exhausted: false,
        }
    }
}

impl<I: Iterator<Item = u16>> Iterator for U16AsUSizeIter<I> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = [0; HEADER_LENGTH];
        if self.exhausted {
            return None;
        }

        let mut any_handled = false;

        for i in 0..HEADER_LENGTH {
            match self.iter.next() {
                Some(val) => {
                    result[i] = val;
                }
                None => {
                    self.exhausted = true;
                    break;
                }
            }
            any_handled = true;
        }

        if !any_handled {
            return None;
        }

        let result: [u16; HEADER_LENGTH] = result;
        let result: usize = unsafe { std::mem::transmute(result) };
        Some(result)
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
        let u16_buffer_length = (length + HEADER_LENGTH - 1) & !(HEADER_LENGTH - 1);

        std::ptr::from_raw_parts_mut::<Self>(start_ptr, u16_buffer_length)
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
    if cef_string.is_null() {
        return None;
    }
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
    let cef_string = unsafe { cef_string.as_ref() }?;
    let bytes = unsafe { std::slice::from_raw_parts(cef_string.str_, cef_string.length) };

    let value = String::from_utf16_lossy(bytes);

    Some(value)
}

#[test]
fn test() {
    let string = "hello world";
    let cef_string = str_into_cef_string_utf16(string);
    let string = unsafe { cef_string_utf16_into_string(&cef_string) };
    assert_eq!(string, Some("hello world".to_string()));

    let string = "another_test_wohoooo";
    let cef_string = str_into_cef_string_utf16(string);
    let string = unsafe { cef_string_utf16_into_string(&cef_string) };
    assert_eq!(string, Some("another_test_wohoooo".to_string()));

    let string = "";
    let cef_string = str_into_cef_string_utf16(string);
    let string = unsafe { cef_string_utf16_into_string(&cef_string) };
    assert_eq!(string, Some("".to_string()));
}
