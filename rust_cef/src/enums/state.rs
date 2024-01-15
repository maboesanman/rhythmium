use cef_wrapper::cef_capi_sys::{
    cef_state_t, cef_state_t_STATE_DEFAULT, cef_state_t_STATE_DISABLED, cef_state_t_STATE_ENABLED,
};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    Default = cef_state_t_STATE_DEFAULT,
    Enabled = cef_state_t_STATE_ENABLED,
    Disabled = cef_state_t_STATE_DISABLED,
}

impl From<State> for cef_state_t {
    fn from(val: State) -> Self {
        val as _
    }
}

impl From<cef_state_t> for State {
    fn from(val: cef_state_t) -> Self {
        match val {
            cef_state_t_STATE_DEFAULT => Self::Default,
            cef_state_t_STATE_ENABLED => Self::Enabled,
            cef_state_t_STATE_DISABLED => Self::Disabled,
            _ => Self::Default,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Default
    }
}
