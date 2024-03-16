use std::collections::HashMap;

use cef_wrapper::cef_capi_sys::{
    cef_base_ref_counted_t, cef_command_line_create, cef_command_line_t, cef_main_args_t,
};

use crate::{
    structs::main_args::MainArgs,
    util::{
        cef_arc::CefArc,
        cef_string::{cef_string_userfree_into_string, str_into_cef_string_utf16},
        starts_with::StartsWith,
    },
};

#[repr(transparent)]
pub struct CommandLine(pub(crate) cef_command_line_t);

unsafe impl StartsWith<cef_command_line_t> for CommandLine {}
unsafe impl StartsWith<cef_base_ref_counted_t> for CommandLine {}
unsafe impl StartsWith<cef_base_ref_counted_t> for cef_command_line_t {}

impl CommandLine {
    pub fn new() -> CefArc<Self> {
        let command_line = unsafe { cef_command_line_create() };
        unsafe { CefArc::from_raw(command_line.cast()) }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new_from_main_args(main_args: MainArgs) -> CefArc<Self> {
        let mut command_line = Self::new();
        command_line
            .try_get_mut()
            .map_err(|_| "Something went horribly wrong!")
            .unwrap()
            .init_from_argv(main_args);
        command_line
    }

    pub fn new_from_env() -> CefArc<Self> {
        #[cfg(not(target_os = "windows"))]
        {
            Self::new_from_main_args(MainArgs::from_env())
        }
        #[cfg(target_os = "windows")]
        {
            unimplemented!()
        }
    }

    #[doc = "\n Returns true if this object is valid. Do not call any other functions\n if this function returns false.\n"]
    pub fn is_valid(&self) -> bool {
        let self_ptr = &self.0 as *const _ as *mut _;
        let is_valid = self.0.is_valid.unwrap();
        unsafe { is_valid(self_ptr) != 0 }
    }

    #[doc = "\n Returns true if the values of this object are read-only. Some APIs may\n expose read-only objects.\n"]
    pub fn is_read_only(&self) -> bool {
        let self_ptr = &self.0 as *const _ as *mut _;
        let is_read_only = self.0.is_read_only.unwrap();
        unsafe { is_read_only(self_ptr) != 0 }
    }

    #[doc = "\n Returns a writable copy of this object.\n"]
    pub fn copy(&self) -> CefArc<Self> {
        unimplemented!()
    }

    #[cfg(not(target_os = "windows"))]
    #[doc = "\n Initialize the command line with the specified arg vector.\n The first argument must be the name of the program."]
    pub fn init_from_argv(&mut self, args: MainArgs) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let init_from_argv = self.0.init_from_argv.unwrap();
        let cef_main_args_t { argc, argv } = args.into();
        let argv = argv as *const *const _;
        unsafe { init_from_argv(self_ptr, argc, argv) }
    }

    #[cfg(target_os = "windows")]
    #[doc = "\n Initialize the command line with the string returned by calling\n GetCommandLineW()."]
    pub fn init_from_string(&mut self, command_line: &str) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let init_from_string = self.0.init_from_string.unwrap();
        let command_line = str_into_cef_string_utf16(command_line);
        unsafe { init_from_string(self_ptr, &command_line) }
    }

    #[doc = "\n Reset the command-line switches and arguments but leave the program\n component unchanged.\n"]
    pub fn reset(&mut self) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let reset = self.0.reset.unwrap();
        unsafe { reset(self_ptr) }
    }

    #[cfg(not(target_os = "windows"))]
    #[doc = "\n Retrieve the original command line string as a vector of strings. The argv\n array: `{ program, [(--|-|/)switch[=value]]*, [--], [argument]* }`\n"]
    pub fn get_argv(&self) -> Vec<String> {
        unimplemented!()
    }

    #[cfg(target_os = "windows")]
    #[doc = "\n Constructs and returns the represented command line string. Use this\n function cautiously because quoting behavior is unclear.\n"]
    pub fn get_command_line_string(&self) -> String {
        unimplemented!()
    }

    #[doc = "\n Get the program part of the command line string (the first item).\n"]
    pub fn get_program(&self) -> String {
        let self_ptr = &self.0 as *const _ as *mut _;
        let get_program = self.0.get_program.unwrap();
        let program = unsafe { get_program(self_ptr) };
        unsafe { cef_string_userfree_into_string(program) }.unwrap()
    }

    #[doc = "\n Set the program part of the command line string (the first item).\n"]
    pub fn set_program(&mut self, program: &str) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let set_program = self.0.set_program.unwrap();
        let program = str_into_cef_string_utf16(program);
        unsafe { set_program(self_ptr, &program) }
    }

    #[doc = "\n Returns true (1) if the command line has switches.\n"]
    pub fn has_switches(&self) -> bool {
        let self_ptr = &self.0 as *const _ as *mut _;
        let has_switches = self.0.has_switches.unwrap();
        unsafe { has_switches(self_ptr) != 0 }
    }

    #[doc = "\n Returns true (1) if the command line contains the given switch.\n"]
    pub fn has_switch(&self, name: &str) -> bool {
        let self_ptr = &self.0 as *const _ as *mut _;
        let has_switch = self.0.has_switch.unwrap();
        let name = str_into_cef_string_utf16(name);
        unsafe { has_switch(self_ptr, &name) != 0 }
    }

    #[doc = "\n Returns the value associated with the given switch. If the switch has no\n value or isn't present this function returns the NULL string.\n"]
    pub fn get_switch_value(&self, name: &str) -> Option<String> {
        let self_ptr = &self.0 as *const _ as *mut _;
        let get_switch_value = self.0.get_switch_value.unwrap();
        let name = str_into_cef_string_utf16(name);
        let value = unsafe { get_switch_value(self_ptr, &name) };
        unsafe { cef_string_userfree_into_string(value) }
    }

    #[doc = "\n Returns the map of switch names and values. If a switch has no value an\n NULL string is returned.\n"]
    pub fn get_switches(&self) -> HashMap<String, String> {
        unimplemented!()
    }

    #[doc = "\n Add a switch to the end of the command line.\n"]
    pub fn append_switch(&mut self, name: &str) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let append_switch = self.0.append_switch.unwrap();
        let name = str_into_cef_string_utf16(name);
        unsafe { append_switch(self_ptr, &name) }
    }

    #[doc = "\n Add a switch with the specified value to the end of the command line. If\n the switch has no value pass an NULL value string.\n"]
    pub fn append_switch_with_value(&mut self, name: &str, value: &str) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let append_switch_with_value = self.0.append_switch_with_value.unwrap();
        let name = str_into_cef_string_utf16(name);
        let value = str_into_cef_string_utf16(value);
        unsafe { append_switch_with_value(self_ptr, &name, &value) }
    }

    #[doc = "\n True if there are remaining command line arguments.\n"]
    pub fn has_arguments(&self) -> bool {
        let self_ptr = &self.0 as *const _ as *mut _;
        let has_arguments = self.0.has_arguments.unwrap();
        unsafe { has_arguments(self_ptr) != 0 }
    }

    #[doc = "\n Get the remaining command line arguments.\n"]
    pub fn get_arguments(&self) -> Vec<String> {
        unimplemented!()
    }

    #[doc = "\n Add an argument to the end of the command line.\n"]
    pub fn append_argument(&mut self, argument: &str) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let append_argument = self.0.append_argument.unwrap();
        let argument = str_into_cef_string_utf16(argument);
        unsafe { append_argument(self_ptr, &argument) }
    }

    #[doc = "\n Insert a command before the current command. Common for debuggers, like\n \"valgrind\" or \"gdb --args\".\n"]
    pub fn prepend_wrapper(&mut self, wrapper: &str) {
        let self_ptr = &self.0 as *const _ as *mut _;
        let prepend_wrapper = self.0.prepend_wrapper.unwrap();
        let wrapper = str_into_cef_string_utf16(wrapper);
        unsafe { prepend_wrapper(self_ptr, &wrapper) }
    }
}

pub enum ProcessType {
    Browser,
    Render,
    Other,
}
impl CommandLine {
    pub fn get_process_type(&self) -> ProcessType {
        let switch_value = self.get_switch_value("kProcessType");
        match switch_value.as_ref().map(String::as_str) {
            None => ProcessType::Browser,
            Some("kRendererProcess") => ProcessType::Render,
            #[cfg(target_os = "linux")]
            Some("kZygoteProcess") => ProcessType::Render,
            Some(_) => ProcessType::Other,
        }
    }
}
