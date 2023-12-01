use cef_sys::{
    cef_run_message_loop,
    cef_quit_message_loop,
    cef_do_message_loop_work,
};

pub fn run_message_loop() {
    unsafe { cef_run_message_loop() };
}

pub fn quit_message_loop() {
    unsafe { cef_quit_message_loop() };
}

pub fn do_message_loop_work() {
    unsafe { cef_do_message_loop_work() };
}