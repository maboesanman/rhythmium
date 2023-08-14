use cef_sys::cef_string_utf16_t;

pub fn cef_string_utf16_to_rust_string(cef_string: *const cef_string_utf16_t) -> String {
    let slice = unsafe {
        let cef_string = cef_string.as_ref().unwrap();
        std::slice::from_raw_parts(cef_string.str_, cef_string.length)
    };

    String::from_utf16_lossy(slice)
}
