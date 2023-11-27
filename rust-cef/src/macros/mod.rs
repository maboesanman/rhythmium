macro_rules! invoke_v_table {
    ($t:ident . $method:ident ()) => {{
        let base = unsafe { &$t.ptr.as_ref().0 };
        let ptr = $t.ptr.as_ptr() as *mut _;
        unsafe { base.$method.unwrap()(ptr) }
    }};
    ($t:ident . $method:ident ( $($arg:expr),* )) => {{
        let base = unsafe { &$t.ptr.as_ref().0 };
        let ptr = $t.ptr.as_ptr() as *mut _;
        unsafe { base.$method.unwrap()(ptr, $($arg),*) }
    }};
}

macro_rules! invoke_mut_v_table {
    ($t:ident . $method:ident ()) => {{
        let base = unsafe { &$t.0.ptr.as_ref().0 };
        let ptr = $t.0.ptr.as_ptr() as *mut _;
        unsafe { base.$method.unwrap()(ptr) }
    }};
    ($t:ident . $method:ident ( $($arg:expr),* )) => {{
        let base = unsafe { &$t.0.ptr.as_ref().0 };
        let ptr = $t.0.ptr.as_ptr() as *mut _;
        unsafe { base.$method.unwrap()(ptr, $($arg),*) }
    }};
}
