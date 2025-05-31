use std::{fmt::Debug, hash::Hash};

mod context;
// pub mod evaluate_to;
mod expire_handle;
mod input_state_manager;
mod output_event_manager;

/// Types for interacting with type erased inputs for a transposer.
pub mod input_erasure;

/// The type that encapsulates the transposer as it updates over time.
pub mod step;

// pub mod evaluate_to;

pub use context::{HandleInputContext, HandleScheduleContext, InitContext, InterpolateContext};
pub use expire_handle::ExpireHandle;

/// A `Transposer` is a type that can update itself in response to events.
///
/// the purpose of this type is to provide an abstraction for game logic which can be used to add rollback and
/// realtime event scheduling, replays, and possibly more.
///
/// it is *heavily* recommended to use immutable structure sharing data types (for example, the [`im`] crate)
/// in the implementing struct, because [`clone`](Clone::clone) is called often and should be a cheap operation.
///
/// Additionally, is is recommended to put any somewhat large, readonly data in an [`Arc`] or [`Rc`], as this will
/// reduce the amount of data that needs to be cloned.
///
/// The name comes from the idea that we are converting a stream of events into another stream of events,
/// perhaps in the way a stream of music notes can be *transposed* into another stream of music notes.
pub trait Transposer: Sized {
    /// The type used as the 'time' for events. This must be Ord and Copy because it is frequently used for comparisons,
    /// and it must be [`Default`] because the default value is used for the timestamp of events emitted.
    /// by the init function.
    type Time: Copy + Ord + Unpin + Debug;

    /// The type of the output payloads.
    ///
    /// The output events are of type `Event<Self::Time, RollbackPayload<Self::Output>>`
    ///
    /// If a rollback must occur which invalidates previously yielded events, an event of type
    /// `Event<Self::Time, RollbackPayload::Rollback>` will be emitted.
    type OutputEvent;

    /// The type of the interpolation.
    ///
    /// This represents the "continuous" game state, and is produced on demand via the interpolate method
    type OutputState;

    /// The type of the payloads of scheduled events
    ///
    /// the events in the schedule are all of type `Event<Self::Time, Self::Scheduled>`
    type Scheduled: Clone;

    /// The function to finalize all inputs and prepare for initialization.
    ///
    /// This function is called after all the supplied inputs' register_input functions have been called.
    /// If the registered inputs are not sufficient for the transposer to operate, this function should return false,
    /// and the transposer will not be initialized.
    ///
    /// If the registered inputs _are_ sufficient, this function should return true.
    fn prepare_to_init(&mut self) -> bool;

    /// The function to initialize your transposer's events.
    ///
    /// You should initialize your transposer like any other struct.
    /// This function is for initializing the schedule events.
    ///
    /// Additionally, this function serves as the validation for the inputs that have been registered.
    /// If this transposer requires a specific input to have been registered, but it was not,
    /// this function should return false.
    ///
    /// `cx` is a context object for performing additional operations.
    /// For more information on `cx` see the [`InitContext`] documentation.
    async fn init(&mut self, cx: &mut InitContext<'_, Self>);

    /// The function to respond to internally scheduled events.
    ///
    /// `time` and `payload` correspond with the event to be handled.
    ///
    /// `cx` is a context object for performing additional operations like scheduling events.
    /// For more information on `cx` see the [`UpdateContext`] documentation.
    async fn handle_scheduled_event(
        &mut self,
        payload: Self::Scheduled,
        cx: &mut HandleScheduleContext<'_, Self>,
    );

    /// The function to interpolate between states
    ///
    /// handle_input and handle_scheduled only operate on discrete times.
    /// If you want the state between two of these times, you have to calculate it.
    ///
    /// `base_time` is the time of the `self` parameter
    /// `interpolated_time` is the time being requested `self`
    /// `cx is a context object for performing additional operations like requesting state.
    async fn interpolate(&self, cx: &mut InterpolateContext<'_, Self>) -> Self::OutputState;
}

/// This represents an input that your transposer expects to be present.
/// This can be a zero-sized type, or a type that contains data.
pub trait TransposerInput: 'static + Sized + Hash + Eq + Copy + Ord {
    /// The base transposer that this input is for.
    type Base: TransposerInputEventHandler<Self>;

    /// The event that this input can emit.
    type InputEvent: Ord;

    /// The state that this input can produce.
    type InputState;

    /// This MUST be unique for each input that shares a base.
    ///
    /// in particular, two inputs with the same Base and SORT, must be of the same type.
    const SORT: u64;
}

/// This trait is for handling input events.
/// You need to implement this trait for your transposer to be able to handle input events.
pub trait TransposerInputEventHandler<I: TransposerInput<Base = Self>>: Transposer {
    /// The function to register an input.
    /// This occurs before the init function is run.
    /// return false if the input is not valid for whatever reason.
    ///
    /// `input` is the specific input.
    ///
    /// `cx` is a context object for performing additional operations like scheduling events.
    /// For more information on `cx` see the [`UpdateContext`] documentation.
    fn register_input(&mut self, input: I) -> bool;

    /// The function to respond to input.
    ///
    /// `input` is the specific input the event is from.
    ///
    /// `event` is the event to be handled.
    ///
    /// `cx` is a context object for performing additional operations like scheduling events.
    /// For more information on `cx` see the [`UpdateContext`] documentation.
    async fn handle_input_event(
        &mut self,
        input: &I,
        event: &I::InputEvent,
        cx: &mut HandleInputContext<'_, Self>,
    );

    /// Filter out events you know you can't do anything with.
    /// This reduces the amount of events you have to remember for rollback to work.
    ///
    /// Note that this has access to very little information. This is meant to be an
    /// optimization, which is why the default implementation is to simply always return `true`
    fn can_handle(time: Self::Time, event: &I::InputEvent) -> bool {
        let _ = (time, event);
        true
    }
}
