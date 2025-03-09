use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::Transposer;

use super::{step::SaturateErr, wrapped_transposer::WrappedTransposer};

/// A step or an init step, used to start saturating the next step.
pub trait PreviousStep<T: Transposer, P: SharedPointerKind> {

    /// get the step's uuid, for debug checks of proper hydration
    #[cfg(debug_assertions)]
    fn get_uuid(&self) -> uuid::Uuid;

    /// remove the wrapped transposer and return it.
    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr>;

    /// clone the wrapped transposer and return it.
    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr>;
}
