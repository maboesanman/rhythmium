use archery::{SharedPointer, SharedPointerKind};

use crate::transposer::Transposer;

use super::{step::SaturateErr, wrapped_transposer::WrappedTransposer};

pub trait PreviousStep<T: Transposer, P: SharedPointerKind> {
    #[cfg(debug_assertions)]
    fn get_uuid(&self) -> uuid::Uuid;

    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr>;

    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr>;
}
