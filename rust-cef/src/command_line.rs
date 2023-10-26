use std::ops::{Deref, DerefMut};

use cef_sys::cef_command_line_t;

// use crate::util::{
//     cef_arc::{CefPtrKindArc, CefArc}, cef_type::{CefType, Unknown},
// };

// pub type CommandLine<RustImpl> = CefType<cef_command_line_t, RustImpl>;
// pub type DynCommandLine = CommandLine<Unknown>;

// // these will be in a macro for the specific ctype/rustimpl combo.
// unsafe impl<CType, RustImpl> CefBase for CefType<CType, RustImpl> {
//     type CType = CType;
//     type Kind = CefPtrKindArc;
// }

// unsafe impl<CType> CefBaseRaw for CType {
//     type RustType = CefType<CType, Unknown>;
//     type Kind = CefPtrKindArc;
// }

// impl<RustImpl> CefArc<CommandLine<RustImpl>> {
//     pub fn is_valid(&self) -> bool {
//         let base = &self.get_base();

//         // unsafe { base.is_valid.unwrap()(base) == 1 }
//         todo!()
//     }
// }

// #[repr(C)]
// pub struct CommandLine<RustImpl> {
//     base: cef_command_line_t,
//     rust_impl: RustImpl,
// }

// pub type DynCommandLine = CommandLine<Unknown>;

// unsafe impl<RustImpl> CefBase for CommandLine<RustImpl> {
//     type CType = cef_command_line_t;
//     type Kind = CefPtrKindArc;
// }

// unsafe impl CefBaseRaw for cef_command_line_t {
//     type RustType = DynCommandLine;
//     type Kind = CefPtrKindArc;
// }
