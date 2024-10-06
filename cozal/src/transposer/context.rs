use core::future::Future;
use std::ptr::NonNull;

use rand_chacha::rand_core::CryptoRngCore;

use super::expire_handle::ExpireHandle;
use super::input_state_manager::{GetInputStateFuture, InputStateManager};
use super::output_event_manager::{EmitOutputFuture, OutputEventManager};
use super::Transposer;
use crate::transposer::TransposerInput;

/// This trait is a supertrait of all the context functionality available to the `Transposer::init` function
pub trait InitContext<'a, T: Transposer>:
    CurrentTimeContext<T>
    + InputStateManagerContext<'a, T>
    + OutputEventManagerContext<T>
    + RngContext
    + ScheduleEventContext<T>
{
}

/// This trait is a supertrait of all the context functionality available to the `Transposer::handle_scheduled` function
pub trait HandleScheduleContext<'a, T: Transposer>:
    CurrentTimeContext<T>
    + ExpireEventContext<T>
    + InputStateManagerContext<'a, T>
    + LastUpdatedTimeContext<T>
    + OutputEventManagerContext<T>
    + RngContext
    + ScheduleEventContext<T>
{
}

/// This trait is a supertrait of all the context functionality available to the `Transposer::interpolate` function
pub trait InterpolateContext<'a, T: Transposer>:
    CurrentTimeContext<T> + LastUpdatedTimeContext<T> + InputStateManagerContext<'a, T>
{
}

/// This trait is a supertrait of all the context functionality available to the `TransposerInputEventHandler::handle_input` function
pub trait HandleInputContext<'a, T: Transposer>:
    CurrentTimeContext<T>
    + ExpireEventContext<T>
    + InputStateManagerContext<'a, T>
    + LastUpdatedTimeContext<T>
    + OutputEventManagerContext<T>
    + RngContext
    + ScheduleEventContext<T>
{
}

/// A trait for accessing the current time (not the system time, but the time this transposer uses)
pub trait CurrentTimeContext<T: Transposer> {
    /// get the current time (either the time of the event currently being processed,
    /// or the time of the interpolation)
    #[must_use]
    fn current_time(&self) -> T::Time;
}

/// A trait for accessing the time of the last processed event (init, input, or scheduled)
pub trait LastUpdatedTimeContext<T: Transposer> {
    /// get the time of the last processed event (init, input, or scheduled)
    /// does not consider events that were filtered out due to `can_handle` returning false.
    #[must_use]
    fn last_updated_time(&self) -> T::Time;
}

/// A trait for accessing the InputStateManager. Not called directly by the user.
#[doc(hidden)]
pub trait InputStateManagerContext<'a, T: Transposer> {
    #[doc(hidden)]
    fn get_input_state_manager(&mut self) -> NonNull<InputStateManager<T>>;
}

/// A trait for accessing the InputStateManager. Not called directly by the user.
#[doc(hidden)]
pub trait OutputEventManagerContext<T: Transposer> {
    #[doc(hidden)]
    fn get_output_event_manager(&mut self) -> NonNull<OutputEventManager<T>>;
}

/// A trait for requesting input state from one of the inputs of this transposer.
pub trait InputStateContext<'a, T: Transposer>: InputStateManagerContext<'a, T> {
    /// get the input state from one of the inputs of this transposer at the current time.
    /// only the specific input state you've requested will be retrieved.
    ///
    /// once the resulting future is awaited, the system will retrieve the input state for the given time from the input soure.
    #[must_use]
    fn get_input_state<'fut, I: TransposerInput<Base = T>>(
        &'fut mut self,
        input: I,
    ) -> GetInputStateFuture<'fut, 'a, I> {
        GetInputStateFuture::new(self.get_input_state_manager(), input)
    }
}

impl<'a, T: Transposer, A: InputStateManagerContext<'a, T>> InputStateContext<'a, T> for A {}

/// A trait for scheduling events. for future processing
pub trait ScheduleEventContext<T: Transposer> {
    /// schedule the an event at `time` with payload `payload`.
    ///
    /// `ScheduleEventError::NewEventBeforeCurrent` will be emitted if the supplied time is
    /// before the current time.
    ///
    /// when using this method, there is no way to expire the event.
    fn schedule_event(
        &mut self,
        time: T::Time,
        payload: T::Scheduled,
    ) -> Result<(), ScheduleEventError>;

    /// schedule the an event at `time` with payload `payload`.
    /// an `ExpireHandle` is returned, which may be stored and later passed to
    /// `ExpireEventContext::expire_event` to remove the event from the schedule.
    ///
    /// `ScheduleEventError::NewEventBeforeCurrent` will be emitted if the supplied time is
    /// before the current time.
    fn schedule_event_expireable(
        &mut self,
        time: T::Time,
        payload: T::Scheduled,
    ) -> Result<ExpireHandle, ScheduleEventError>;
}

#[non_exhaustive]
#[derive(Debug)]
pub enum ScheduleEventError {
    NewEventBeforeCurrent,
}

/// a trait to expire previously scheduled events.
pub trait ExpireEventContext<T: Transposer> {
    /// expire the event corresponding to the supplied `ExpireHandle`
    ///
    /// if there is no corresponding event, `ExpireEventError::InvalidOrUsedHandle` will be emitted.
    fn expire_event(
        &mut self,
        handle: ExpireHandle,
    ) -> Result<(T::Time, T::Scheduled), ExpireEventError>;
}

#[non_exhaustive]
#[derive(Debug)]
pub enum ExpireEventError {
    InvalidOrUsedHandle,
}

/// A trait to emit events to the output.
pub trait EmitEventContext<T: Transposer> {
    /// Emit an event. this emits the event at the current time.
    ///
    /// If you'd like to emit an event in the future, schedule an event at that time, and emit the event
    /// when you handle that scheduled event.
    ///
    /// the event is not emitted until the future is awaited.
    #[must_use]
    fn emit_event<'a>(&'a mut self, payload: T::OutputEvent) -> impl Future<Output = ()> + 'a
    where
        T: 'a;
}

impl<C: ?Sized, T: Transposer> EmitEventContext<T> for C
where
    C: OutputEventManagerContext<T>,
{
    fn emit_event<'a>(&'a mut self, payload: T::OutputEvent) -> impl Future<Output = ()> + 'a
    where
        T: 'a,
    {
        EmitOutputFuture::new(self.get_output_event_manager(), payload)
    }
}

/// A trait to deterministically produce randomness.
pub trait RngContext {
    /// Get access to the `RngCore` for use in the transposer.
    ///
    /// This should be the only source of entropy used in your transposer.
    ///
    /// This is a Cryptographically secure PRNG. If you want speed over security,
    /// and use a LOT of randomness, you can use this to seed a cheaper PRNG, and store that yourself
    /// in your init function.
    #[must_use]
    fn get_rng(&mut self) -> &mut dyn CryptoRngCore;
}
