use cef_sys::cef_base_ref_counted_t;



unsafe extern "C" fn add_ref(base: *mut cef_base_ref_counted_t) {
}

unsafe extern "C" fn release(base: *mut cef_base_ref_counted_t) -> std::os::raw::c_int {
    1
}

unsafe extern "C" fn has_one_ref(base: *mut cef_base_ref_counted_t) -> std::os::raw::c_int {
    1
}

unsafe extern "C" fn has_at_least_one_ref(base: *mut cef_base_ref_counted_t) -> std::os::raw::c_int {
    1
}

pub fn initialize_cef_base_refcounted(base: *mut cef_base_ref_counted_t) {
    unsafe {
        if (*base).size <= 0 {
            panic!("size not set");
        }
        (*base).add_ref = Some(add_ref);
        (*base).release = Some(release);
        (*base).has_one_ref = Some(has_one_ref);
        (*base).has_at_least_one_ref = Some(has_at_least_one_ref);
    }
}