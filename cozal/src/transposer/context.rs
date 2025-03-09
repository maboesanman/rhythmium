use core::future::Future;
use std::ptr::NonNull;

use rand_chacha::rand_core::CryptoRngCore;

use super::expire_handle::ExpireHandle;
use super::input_state_manager::{GetInputStateFuture, InputStateManager};
use super::output_event_manager::{EmitOutputFuture, OutputEventManager};
use super::Transposer;
use crate::transposer::TransposerInput;

/// A trait for accessing the current time (not the system time, but the time this transposer uses)
pub trait CurrentTimeContext<T: Transposer> {
    #[must_use]
    fn current_time(&self) -> T::Time;
}

/// A trait for accessing the time of the last processed event (init, input, or scheduled)
pub trait LastUpdatedTimeContext<T: Transposer> {
    /// get the time of the last processed event (init, input, or scheduled)
    /// does not consider events that were filtered out due to `can_handle` returning false.
    #[must_use]
    fn last_updated_time(&self) -> Option<T::Time>;
}
/// A trait for accessing the InputStateManager. Not called directly by the user.
pub trait InputStateManagerContext<'a, T: Transposer> {
    fn get_input_state_manager(&mut self) -> NonNull<InputStateManager<T>>;
}

/// A trait for accessing the InputStateManager. Not called directly by the user.
pub trait OutputEventManagerContext<T: Transposer> {
    fn get_output_event_manager(&mut self) -> NonNull<OutputEventManager<T>>;
}

/// A trait for scheduling events. for future processing
pub trait ScheduleEventContext<T: Transposer> {
    fn schedule_event(
        &mut self,
        time: T::Time,
        payload: T::Scheduled,
    ) -> Result<(), ScheduleEventError>;

    fn schedule_event_expireable(
        &mut self,
        time: T::Time,
        payload: T::Scheduled,
    ) -> Result<ExpireHandle, ScheduleEventError>;
}

/// A trait for scheduling events. for future processing
pub trait ScheduleEventContextInfallible<T: Transposer> {
    fn schedule_event(&mut self, time: T::Time, payload: T::Scheduled);

    fn schedule_event_expireable(&mut self, time: T::Time, payload: T::Scheduled) -> ExpireHandle;
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

/// A trait to deterministically produce randomness.
pub trait RngContext {
    fn get_rng(&mut self) -> &mut dyn CryptoRngCore;
}

macro_rules! impl_single {
    (CurrentTimeContext) => {
        /// get the current time (either the time of the event currently being processed,
        /// or the time of the interpolation)
        pub fn current_time(&self) -> T::Time {
            self.0.current_time()
        }
    };
    (LastUpdatedTimeContext) => {
        /// get the time of the last processed event (init, input, or scheduled)
        /// does not consider events that were filtered out due to `can_handle` returning false.
        pub fn last_updated_time(&self) -> Option<T::Time> {
            self.0.last_updated_time()
        }
    };
    (InputStateManagerContext) => {
        /// get the input state from one of the inputs of this transposer at the current time.
        /// only the specific input state you've requested will be retrieved.
        ///
        /// once the resulting future is awaited, the system will retrieve the input state
        /// for the given time from the input soure.
        pub fn get_input_state<'fut, I: TransposerInput<Base = T>>(
            &'fut mut self,
            input: I,
        ) -> impl 'fut + Future<Output = &'a I::InputState> {
            GetInputStateFuture::new(self.0.get_input_state_manager(), input)
        }
    };
    (OutputEventManagerContext) => {
        /// Emit an event. this emits the event at the current time.
        ///
        /// If you'd like to emit an event in the future, schedule an event at that time, and emit the event
        /// when you handle that scheduled event.
        ///
        /// the event is not emitted until the future is awaited.
        pub fn emit_event(&mut self, payload: T::OutputEvent) -> impl '_ + Future<Output = ()> {
            EmitOutputFuture::new(self.0.get_output_event_manager(), payload)
        }
    };
    (ScheduleEventContext) => {
        /// schedule the an event at `time` with payload `payload`.
        ///
        /// `ScheduleEventError::NewEventBeforeCurrent` will be emitted if the supplied time is
        /// before the current time.
        ///
        /// when using this method, there is no way to expire the event.
        pub fn schedule_event(
            &mut self,
            time: T::Time,
            payload: T::Scheduled,
        ) -> Result<(), ScheduleEventError> {
            self.0.schedule_event(time, payload)
        }

        /// schedule the an event at `time` with payload `payload`.
        /// an `ExpireHandle` is returned, which may be stored and later passed to
        /// `ExpireEventContext::expire_event` to remove the event from the schedule.
        ///
        /// `ScheduleEventError::NewEventBeforeCurrent` will be emitted if the supplied time is
        /// before the current time.
        pub fn schedule_event_expireable(
            &mut self,
            time: T::Time,
            payload: T::Scheduled,
        ) -> Result<ExpireHandle, ScheduleEventError> {
            self.0.schedule_event_expireable(time, payload)
        }
    };
    (ScheduleEventContextInfallible) => {
        /// schedule the an event at `time` with payload `payload`.
        ///
        /// `ScheduleEventError::NewEventBeforeCurrent` will be emitted if the supplied time is
        /// before the current time.
        ///
        /// when using this method, there is no way to expire the event.
        pub fn schedule_event(&mut self, time: T::Time, payload: T::Scheduled) {
            self.0.schedule_event(time, payload)
        }

        /// schedule the an event at `time` with payload `payload`.
        /// an `ExpireHandle` is returned, which may be stored and later passed to
        /// `ExpireEventContext::expire_event` to remove the event from the schedule.
        ///
        /// `ScheduleEventError::NewEventBeforeCurrent` will be emitted if the supplied time is
        /// before the current time.
        pub fn schedule_event_expireable(
            &mut self,
            time: T::Time,
            payload: T::Scheduled,
        ) -> ExpireHandle {
            self.0.schedule_event_expireable(time, payload)
        }
    };
    (ExpireEventContext) => {
        /// expire the event corresponding to the supplied `ExpireHandle`
        ///
        /// if there is no corresponding event, `ExpireEventError::InvalidOrUsedHandle` will be emitted.
        pub fn expire_event(
            &mut self,
            handle: ExpireHandle,
        ) -> Result<(T::Time, T::Scheduled), ExpireEventError> {
            self.0.expire_event(handle)
        }
    };
    (RngContext) => {
        /// Get access to the `RngCore` for use in the transposer.
        ///
        /// This should be the only source of entropy used in your transposer.
        ///
        /// This is a Cryptographically secure PRNG. If you want speed over security,
        /// and use a LOT of randomness, you can use this to seed a cheaper PRNG, and store that yourself
        /// in your init function.
        #[must_use]
        pub fn get_rng(&mut self) -> &mut dyn CryptoRngCore {
            self.0.get_rng()
        }
    };
}

/// A struct for accessing the functions available to the `Transposer::init` function
#[repr(transparent)]
pub struct InitContext<'a, T: Transposer>(dyn InitContextInner<'a, T>);

pub trait InitContextInner<'a, T: Transposer>:
    RngContext + ScheduleEventContextInfallible<T>
{
}

impl<'a, T: Transposer> InitContext<'a, T> {
    pub(crate) fn new_mut<'b>(inner: &'b mut dyn InitContextInner<'a, T>) -> &'b mut Self {
        unsafe { core::mem::transmute(inner) }
    }

    impl_single!(RngContext);
    impl_single!(ScheduleEventContextInfallible);
}

/// A struct for accessing the functions available to the `Transposer::handle_scheduled_event` function
#[repr(transparent)]
pub struct HandleScheduleContext<'a, T: Transposer>(dyn HandleScheduleContextInner<'a, T>);

pub trait HandleScheduleContextInner<'a, T: Transposer>:
    CurrentTimeContext<T>
    + ExpireEventContext<T>
    + LastUpdatedTimeContext<T>
    + RngContext
    + ScheduleEventContext<T>
    + InputStateManagerContext<'a, T>
    + OutputEventManagerContext<T>
{
}

impl<'a, T: Transposer> HandleScheduleContext<'a, T> {
    pub(crate) fn new_mut<'b>(
        inner: &'b mut dyn HandleScheduleContextInner<'a, T>,
    ) -> &'b mut Self {
        unsafe { core::mem::transmute(inner) }
    }

    impl_single!(CurrentTimeContext);
    impl_single!(ExpireEventContext);
    impl_single!(LastUpdatedTimeContext);
    impl_single!(RngContext);
    impl_single!(ScheduleEventContext);
    impl_single!(InputStateManagerContext);
    impl_single!(OutputEventManagerContext);
}

/// A struct for accessing the functions available to the `TransposerInputHandler::handle_input` function
#[repr(transparent)]
pub struct HandleInputContext<'a, T: Transposer>(dyn HandleInputContextInner<'a, T>);

pub trait HandleInputContextInner<'a, T: Transposer>:
    CurrentTimeContext<T>
    + ExpireEventContext<T>
    + InputStateManagerContext<'a, T>
    + LastUpdatedTimeContext<T>
    + OutputEventManagerContext<T>
    + RngContext
    + ScheduleEventContext<T>
{
}

impl<'a, T: Transposer> HandleInputContext<'a, T> {
    pub(crate) fn new_mut<'b>(inner: &'b mut dyn HandleInputContextInner<'a, T>) -> &'b mut Self {
        unsafe { core::mem::transmute(inner) }
    }

    impl_single!(CurrentTimeContext);
    impl_single!(ExpireEventContext);
    impl_single!(InputStateManagerContext);
    impl_single!(LastUpdatedTimeContext);
    impl_single!(OutputEventManagerContext);
    impl_single!(RngContext);
    impl_single!(ScheduleEventContext);
}

/// A struct for accessing the functions available to the `Transposer::interpolate` function
#[repr(transparent)]
pub struct InterpolateContext<'a, T: Transposer>(dyn InterpolateContextInner<'a, T>);

pub trait InterpolateContextInner<'a, T: Transposer>:
    CurrentTimeContext<T> + InputStateManagerContext<'a, T> + LastUpdatedTimeContext<T>
{
}

impl<'a, T: Transposer> InterpolateContext<'a, T> {
    pub(crate) fn new_mut<'b>(inner: &'b mut dyn InterpolateContextInner<'a, T>) -> &'b mut Self {
        unsafe { core::mem::transmute(inner) }
    }

    impl_single!(CurrentTimeContext);
    impl_single!(InputStateManagerContext);
    impl_single!(LastUpdatedTimeContext);
}
