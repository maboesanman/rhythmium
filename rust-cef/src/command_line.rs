use cef_sys::{cef_command_line_create, cef_command_line_t};

use crate::util::cef_string::{into_cef_str_utf16, into_string};
use crate::util::cef_type::VTable;

use crate::util::cef_arc::{CefArc, CefArcMut, VTableKindArc};

#[repr(transparent)]
pub struct CommandLine(cef_command_line_t);

unsafe impl VTable for CommandLine {
    type Kind = VTableKindArc;
}

impl CommandLine {
    pub fn new() -> CefArc<Self> {
        unsafe { CefArc::from_mut_ptr(unsafe { cef_command_line_create() }.cast()) }
    }
}

impl CefArc<CommandLine> {
    pub fn is_valid(&self) -> bool {
        invoke_v_table!(self.is_valid()) == 1
    }

    pub fn is_read_only(&self) -> bool {
        invoke_v_table!(self.is_read_only()) == 1
    }

    pub fn copy(&self) -> Self {
        let new = invoke_v_table!(self.copy()).cast();
        unsafe { CefArc::from_mut_ptr(new) }
    }

    pub fn into_mut(self) -> CefArcMut<CommandLine> {
        match self.try_into_mut() {
            Ok(arc_mut) => return arc_mut,
            Err(this) => {
                let new = invoke_v_table!(this.copy()).cast::<CommandLine>();
                unsafe { CefArc::from_mut_ptr(new).into_mut() }
            }
        }
    }

    pub fn get_program(&self) -> String {
        let result = invoke_v_table!(self.get_program());
        into_string(result).unwrap()
    }

    pub fn has_switches(&self) -> bool {
        invoke_v_table!(self.has_switches()) == 1
    }

    pub fn has_switch(&self, name: &str) -> bool {
        let name = into_cef_str_utf16(name);
        invoke_v_table!(self.has_switch(&name)) == 1
    }

    pub fn get_switch_value(&self, name: &str) -> Option<String> {
        let name = into_cef_str_utf16(name);
        let result = invoke_v_table!(self.get_switch_value(&name));
        into_string(result)
    }

    pub fn get_switches(&self) {
        todo!()
    }

    pub fn has_arguments(&self) -> bool {
        invoke_v_table!(self.has_arguments()) == 1
    }

    pub fn get_arguments(&self) {
        todo!()
    }
}

impl CefArcMut<CommandLine> {
    pub fn reset(&self) {
        invoke_mut_v_table!(self.reset())
    }

    pub fn set_program(&self) {
        todo!()
    }

    pub fn append_switch(&self, name: &str) {
        let name = into_cef_str_utf16(name);
        invoke_mut_v_table!(self.append_switch(&name))
    }

    pub fn append_switch_with_value(&self, name: &str, value: &str) {
        let name = into_cef_str_utf16(name);
        let value = into_cef_str_utf16(value);
        invoke_mut_v_table!(self.append_switch_with_value(&name, &value))
    }

    pub fn append_argument(&self, argument: &str) {
        let argument = into_cef_str_utf16(argument);
        invoke_mut_v_table!(self.append_argument(&argument))
    }

    pub fn prepend_wrapper(&self, wrapper: &str) {
        let wrapper = into_cef_str_utf16(wrapper);
        invoke_mut_v_table!(self.prepend_wrapper(&wrapper))
    }
}

#[cfg(not(target_os = "windows"))]
impl CefArc<CommandLine> {
    pub fn init_from_argv(
        &self,
        argc: std::os::raw::c_int,
        argv: *const *const std::os::raw::c_char,
    ) {
        invoke_v_table!(self.init_from_argv(argc, argv))
    }

    pub fn get_argv(&self) -> Vec<String> {
        todo!()
    }
}

#[cfg(target_os = "windows")]
impl CefArc<CommandLine> {
    pub fn init_from_string(&self, command_line: &str) {
        todo!()
    }

    pub fn get_command_line_string(&self) -> String {
        todo!()
    }
}
