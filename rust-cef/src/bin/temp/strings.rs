use cef_sys::cef_string_utf16_t;

pub fn into_cef_str(string: &str) -> cef_string_utf16_t {
    let bytes = string.encode_utf16().collect::<Vec<_>>();
    let bytes = bytes.into_boxed_slice();

    let (str_, length) = Box::into_raw(bytes).to_raw_parts();

    let str_ = str_.cast();

    unsafe extern "C" fn drop_string(ptr: *mut u16) {
        let _ = ptr;
        todo!()
    }

    cef_string_utf16_t {
        str_,
        length,
        dtor: Some(drop_string),
    }
}
