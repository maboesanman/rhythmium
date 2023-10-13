#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[test]
pub fn test() {
    unsafe {
        cef_base64encode(std::ptr::null(), 0);
    }
}
