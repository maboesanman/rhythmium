use cef_sys::{cef_string_userfree_t, cef_string_utf16_t};

const HEADER_BYTES: usize = std::mem::size_of::<usize>();
const HEADER_LENGTH: usize = HEADER_BYTES >> 1;

unsafe extern "C" fn drop_string(ptr: *mut u16) {
    let ptr = unsafe { ptr.byte_sub(HEADER_BYTES).cast::<usize>() };
    let length = unsafe { *ptr };

    let length = (length + 2 * HEADER_LENGTH - 1) / HEADER_LENGTH;

    let _ = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(ptr, length)) };
}

pub fn str_into_cef_string_utf16(string: &str) -> cef_string_utf16_t {
    let mut buffer = Vec::new();
    buffer.push(0_usize);

    // the data
    let mut array_chunks = string.encode_utf16().array_chunks::<HEADER_LENGTH>();

    for chunk in array_chunks.by_ref() {
        let chunk = unsafe { core::mem::transmute::<[u16; HEADER_LENGTH], usize>(chunk) };
        buffer.push(chunk);
    }

    let remainder = array_chunks.into_remainder();

    let mut last_usize_fill_level = 0;

    if let Some(remainder) = remainder {
        let mut last = [0_u16; HEADER_LENGTH];
        for value in remainder {
            last[last_usize_fill_level] = value;
            last_usize_fill_level += 1;
        }
        let last = unsafe { core::mem::transmute::<[u16; HEADER_LENGTH], usize>(last) };
        buffer.push(last);
    } else {
        last_usize_fill_level = HEADER_LENGTH;
    }

    let num_empty_trailing_u16 = HEADER_LENGTH - last_usize_fill_level;

    // collect the data into a single allocation
    let length = buffer.len();
    let length = (length - 1) * HEADER_LENGTH - num_empty_trailing_u16;
    buffer[0] = length;

    let all_data = Box::into_raw(buffer.into_boxed_slice());
    let str_ = unsafe { all_data.byte_add(HEADER_BYTES) }.cast();

    cef_string_utf16_t {
        str_,
        length,
        dtor: Some(drop_string),
    }
}

pub fn path_into_cef_string_utf16(path: &std::path::Path) -> cef_string_utf16_t {
    let string = path.to_string_lossy();
    str_into_cef_string_utf16(&string)
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
    let cef_string = unsafe { cef_string.as_ref() }?;
    let bytes = unsafe { std::slice::from_raw_parts(cef_string.str_, cef_string.length) };

    let value = String::from_utf16_lossy(bytes);

    Some(value)
}
