use cef_sys::{cef_string_userfree_t, cef_string_utf16_t};

pub fn into_cef_str_utf16(string: &str) -> cef_string_utf16_t {
    let bytes = string.encode_utf16().collect::<Vec<_>>();
    let bytes = bytes.into_boxed_slice();

    let (str_, length) = Box::into_raw(bytes).to_raw_parts();

    let str_ = str_.cast();

    unsafe extern "C" fn drop_string(ptr: *mut u16) {
        todo!()
    }

    cef_string_utf16_t {
        str_,
        length,
        dtor: Some(drop_string),
    }
}

pub fn into_string(cef_string: cef_string_userfree_t) -> Option<String> {
    let boxed = unsafe { Box::from_raw(cef_string) };
    if boxed.str_.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(boxed.str_, boxed.length) };

    let value = String::from_utf16_lossy(bytes);

    unsafe { boxed.dtor.unwrap()(boxed.str_) };

    return Some(value);
}
