I'm trying to build a library on which i will build some rhythm games.

The library does a few things:
- support out of order handling of "events" (could be the beginning of a note's jugement window, the end of a note's jugement window, a keypress from the player, etc)
- support rollback of events
- support "interpolated" state, which is used when determining what a frame should look like (where exactly should you draw the arrow in ddr for example)
- variable timestep (the game should "jump" to the next event, not wait a specific constant interval)

The architecture generally works like this:

one important type is the `Source` type, which can be thought of as a combination of a "Stream" of interrupts which can be polled for state at a specific time. like streams there are combinator functions which may combine sources, transform sources, etc. Eventually i will have a scene editor where users can pipe sources together to create a playable game.

here is the trait definition for `Source`:
```rust
/// An interface for querying partially complete sources of [states](`Source::State`) and [events](`Source::Events`)
///
/// The [`Source`] trait is the core abstraction for the entire cozal library. Everything is designed around the idea of making chains of [`Source`]s
///
/// When a type implements Source, it models two things:
///
/// - A timestamped set of events
/// - A function (in the mathematical sense) mapping `Source::Time` to `Source::State`
/// 
/// Generally, the source is used by polling for a state, and being interrupted with events you must handle. You may also be informed of previously emitted states/events being invalidated.
pub trait Source {
    /// The type used for timestamping events and states.
    type Time: Ord + Copy;

    /// The type of events emitted by the source.
    type Event;

    /// The type of states emitted by the source.
    type State;

    /// Attempt to retrieve the state of the source at `time`, registering the current task for wakeup in certain situations.
    ///
    /// # Return value
    ///
    /// There are several possible return values, each indicating a distinct source state for a time `t`:
    ///
    /// - `SourcePoll::Ready { state, next_event_at }` indicates the source has produced a state for `t`. `state` is the state produced, and `next_event_at` is the time of the next event, if known. If `next_event_at` is `None`, the source does not know about any events after `t`, and the caller need not poll again until the interrupt waker is woken. The source MUST wake the caller when next_event_at would change (From None to Some, Some(x) to Some(y), or Some to None). If the source knows about one future event at t1 and returns Some(t1),then later finds out about another at t2 after the first, it doesn't need to wake the caller since both scenarios would return Some(t1).
    /// 
    /// - `SourcePoll::Interrupt { time, interrupt }` indicates the source has produced an interrupt that must be handled before the state for time `t` can be calculated/emitted. `time` is the time of the interrupt (which must be less than or equal to t), and `interrupt` is the interrupt itself. The caller may do whatever they want with the interrupt but they must  The source MUST wake the caller when the interrupt waker is woken.
    /// 
    /// - `SourcePoll::Pending` indicates the source is not ready to return a state or an interrupt, and will wake on one of the provided wakers when progress is able to be made. If a source returns pending, it is expected not to undo or discard any progress when polled on different channels, no matter what time they poll on. Interrupts are channel-less, and wake the most recently provided interrupt waker. State progress is channel specific, and wakes the state waker.
    /// 
    /// polling with the same channel but a different time when the previous state hasn't returned yet may undo or throw away progress. The caller is expected to continue polling for the same exact time (per channel) until a state is available.
    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State>;

    /// Attempt to retrieve the state of the source at `time`, registering the current task for wakeup in certain situations. Also inform the source that the state emitted from this call is exempt from the requirement to be informed of future invalidations (that the source can "forget" about this call to poll when determining how far to roll back).
    ///
    /// If you do not need to be notified that this state has been invalidated (if for example you polled in order to render to the screen, so finding out your previous frame was wrong means nothing because you can't go back and change it) then this function should be preferred.
    /// 
    /// The emitted interrupts from this method still need to be considered invalid when rolled back. only the state invalidation is affected by the choice to use this over `poll`
    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        self.poll(time, cx)
    }

    /// Attempt to determine information about the set of events before `time` without generating a state. this function behaves the same as [`poll_forget`](Source::poll_forget) but returns `()` instead of [`State`](Source::State). This function should be used in all situations when the state is not actually needed, as the implementer of the trait may be able to do less work.
    ///
    /// If you do not need to use the state, this should be preferred over poll. For example, if you are simply verifying the source does not have new events before a time t, poll_ignore_state could be faster than poll (with a custom implementation).
    fn poll_events(
        &mut self,
        time: Self::Time,
        interrupt_waker: Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()>;

    /// Inform the source it is no longer obligated to retain progress made on `channel`
    fn release_channel(&mut self, channel: usize);

    /// Inform the source that you (the caller) will never poll before `time` again on any channel.
    ///
    /// Calling poll before this time should result in `SourcePollError::PollAfterAdvance`
    fn advance(&mut self, time: Self::Time);

    /// The maximum value which can be used as the channel for a poll call.
    ///
    /// all channels in 0..max_channel() are valid (note that 0 is always an option)
    fn max_channel(&mut self) -> NonZeroUsize;
}

/// information on how to wake the caller, and the obligations of the source to wake the caller.
#[derive(Clone)]
pub struct SourceContext {
    /// The channel the source is currently polling on
    pub channel: usize,

    /// The waker to wake the caller when the source may make progress generating the requested state
    pub channel_waker: Waker,

    /// The waker to wake the caller when the source may produce an interrupt
    pub interrupt_waker: Waker,
}

/// A modified version of [`futures::task::Poll`] For sources
pub enum SourcePoll<T, E, S> {
    /// Indicates the poll is complete
    Ready {
        /// The requested state
        state: S,
        /// The time of the next known event, if known.
        /// 
        /// This is the mechanism the source uses to let the caller sleep until the next event is available.
        next_event_at: Option<T>,
    },

    /// Indicates information must be handled before state is emitted
    Interrupt {
        /// The time the interrupt occurs
        time: T,

        /// The value of the interrupt
        interrupt: Interrupt<E>,
    },

    /// pending operation. caller will be woken up when progress can be made
    /// the channel this poll used must be retained.
    Pending,
}

/// The type of interrupt emitted from the source
pub enum Interrupt<E> {
    /// A new event is available.
    Event(E),

    /// An event followed by a finalize, for convenience.
    /// This should be identical to returning an event then a finalize for the same time.
    /// Useful for sources which never emit Rollbacks, so they can simply emit this interrupt
    /// for every event and nothing else.
    FinalizedEvent(E),

    /// All events previously emitted at or after time T must be discarded.
    /// 
    /// All states previously produced by `poll(t)` (not `poll_forget`) where t is at or after time T must be discarded.
    Rollback,
  
    /// No event will ever be emitted before time T again.
    /// 
    /// This is critical for callers to determine when they can drop certain events they may have been holding on to in case of interrupts.
    Finalize,
}

#[non_exhaustive]
pub enum SourcePollErr<T> {
    OutOfBoundsChannel,
    PollAfterAdvance { advanced: T },
    PollBeforeDefault,
    SpecificError(anyhow::Error),
}

pub type TrySourcePoll<T, E, S> = Result<SourcePoll<T, E, S>, SourcePollErr<T>>;
```

I am working on creating a specific Source implementation that uses another trait to describe logic for combining and responding to multiple input sources.

The trait is called `Transposer`

```rust
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
    type Time: Copy + Ord + Unpin;

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
```

Transposers are wrapped up in something called a `Step`, which allows us to encalsulate the transposer logic in a consistent way.

```rust
/// A step is a structure that allows for the transposer to be thought of as a state machine.
///
/// A step represents the change that occurs to the transposer at a single point in time.
///
/// A step can be in one of three states:
///
/// - Unsaturated: The step is ready to recieve a transposer (and some additional metadata like the scheduled events)
///   and begin processing it.
///
/// - Saturating: The step is in the process of saturating. This means there is some async method on the transposer
///   that has not yet completed. This could be a future that is waiting on some input.
///
/// - Saturated: The step has completed saturating. This means that all async methods on the transposer have completed,
///   and that the transposer is available to either perform interpolation or to be used in the next step.
///
/// A step can move between the states in the following ways:
///
/// - Unsaturated -> Saturating: When the `start_saturate_clone` or `start_saturate_take` methods are called.
///
/// - Saturating -> Saturated: When polling the step returns `Poll::Ready`.
///
/// - (Saturating or Saturated) -> Unsaturated: When the `desaturate` method is called.
///
/// - Saturated -> Unsaturated: When the `start_saturate_take` method is called on the _next_ step.
///
/// Steps are only created by calling `new_init` (at the very beginning to get things started) or by calling
/// `next_unsaturated` or `next_scheduled_unsaturated` on an existing step.
#[derive(Debug)]
pub struct Step<'t, T: Transposer + 't, P: SharedPointerKind + 't = ArcTK> {
    sequence_number: usize,

    steps: Vec<BoxedSubStep<'t, T, P>>,
    status: StepStatus,

    time: T::Time,

    // this is considered the owner of the input state.
    // we are responsible for dropping it.
    shared_step_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)>,
    event_count: usize,
    can_produce_events: bool,

    #[cfg(debug_assertions)]
    uuid_self: uuid::Uuid,
    #[cfg(debug_assertions)]
    uuid_prev: Option<uuid::Uuid>,
}

impl<'a, T: Transposer + 'a, P: SharedPointerKind + 'a> Drop for Step<'a, T, P> {
    fn drop(&mut self) {
        // doesn't matter if there are non-null pointers to this in steps since they won't access this during drop.
        Self::drop_shared_step_state(self.shared_step_state);
    }
}

impl<'a, T: Transposer + 'a, P: SharedPointerKind + 'a> Step<'a, T, P> {
    fn drop_shared_step_state(ptr: NonNull<(OutputEventManager<T>, InputStateManager<T>)>) {
        unsafe { NonNull::drop_in_place(ptr) }
    }

    fn new_shared_step_state() -> NonNull<(OutputEventManager<T>, InputStateManager<T>)> {
        let input_state = Box::default();
        NonNull::from(Box::leak(input_state))
    }

    fn get_step_status_ref(&self) -> ActiveStepStatusRef<'_, T, P> {
        match self.status {
            StepStatus::Unsaturated => ActiveStepStatusRef::Unsaturated,
            StepStatus::Saturating(_) => ActiveStepStatusRef::Saturating,
            StepStatus::Saturated => {
                let step = self.steps.last().unwrap();
                let transposer = step.as_ref().get_finished_transposer().unwrap();
                ActiveStepStatusRef::Saturated(transposer)
            }
        }
    }

    fn get_step_status_mut(&mut self) -> ActiveStepStatusMut<'_, 'a, T, P> {
        match self.status {
            StepStatus::Unsaturated => {
                let step = self.steps.first_mut().unwrap();
                ActiveStepStatusMut::Unsaturated(step)
            }
            StepStatus::Saturating(i) => {
                let step = self.steps.get_mut(i).unwrap();
                ActiveStepStatusMut::Saturating(step)
            }
            StepStatus::Saturated => {
                let step = self.steps.last_mut().unwrap();
                ActiveStepStatusMut::Saturated(step)
            }
        }
    }

    fn get_input_state_mut(&mut self) -> &mut InputStateManager<T> {
        &mut unsafe { self.shared_step_state.as_mut() }.1
    }

    fn get_output_state_mut(&mut self) -> &mut OutputEventManager<T> {
        let mut input_state: NonNull<(OutputEventManager<T>, InputStateManager<T>)> =
            self.shared_step_state;
        &mut unsafe { input_state.as_mut() }.0
    }

    pub fn get_sequence_number(&self) -> usize {
        self.sequence_number
    }

    /// Create new beginning step.
    ///
    /// This is the first step the transposer undergoes, whic his why it recieves the transposer as an argument, as
    /// opposed to the other steps which get it from the previous step.
    pub fn new_init(
        transposer: T,
        pre_init_step: PreInitStep<T>,
        start_time: T::Time,
        rng_seed: [u8; 32],
    ) -> Result<Self, T>
    where
        T: Clone,
    {
        let shared_step_state = Self::new_shared_step_state();
        let uuid_self = uuid::Uuid::new_v4();
        let uuid_prev = None;

        let transposer = pre_init_step.execute(transposer)?;
        let init_sub_step =
            InitSubStep::new_boxed(transposer, rng_seed, start_time, shared_step_state);

        Ok(Self {
            sequence_number: 0,
            steps: vec![init_sub_step],
            status: StepStatus::Saturating(0),
            time: start_time,
            shared_step_state,
            event_count: 0,
            can_produce_events: true,
            #[cfg(debug_assertions)]
            uuid_self,
            #[cfg(debug_assertions)]
            uuid_prev,
        })
    }

    /// Create a new step that is ready to be saturated.
    ///
    /// This will compare the time of the next scheduled event in the current schedule with the time
    /// of `next_inputs`, and either take the next input event from the container to produce a step, or
    /// leave it in the container and produce a step that will handle the scheduled event.
    pub fn next_unsaturated<F: FutureInputContainer<'a, T, P>>(
        &self,
        next_inputs: &mut F,
    ) -> Result<Option<Self>, NextUnsaturatedErr>
    where
        T: Clone,
    {
        let wrapped_transposer = match self.get_step_status_ref() {
            ActiveStepStatusRef::Saturated(t) => t,
            _ => return Err(NextUnsaturatedErr::NotSaturated),
        };

        let next_scheduled_time = wrapped_transposer
            .metadata
            .get_next_scheduled_time()
            .map(|t| t.time);

        let next_input = next_inputs.next();

        if let Some(i) = next_input.as_ref() {
            if i.get_time() <= self.time {
                return Err(NextUnsaturatedErr::InputPastOrPresent);
            }
        }

        let (time, next_scheduled_time, next_input) = match (next_scheduled_time, next_input) {
            (None, None) => return Ok(None),
            (None, Some(i)) => (i.get_time(), None, Some(i)),
            (Some(t), None) => (t, Some(t), None),
            (Some(t), Some(i)) => {
                let i_time = i.get_time();
                if i_time > t {
                    (t, Some(t), None)
                } else {
                    (i_time, None, Some(i))
                }
            }
        };

        let steps = match (next_scheduled_time, next_input) {
            (None, Some(i)) => {
                let mut steps = Vec::new();
                let mut front = Some(i);
                loop {
                    let (item, new_front) = match front.take() {
                        Some(front) => {
                            if front.get_time() != time {
                                break;
                            }
                            front.take_sub_step()
                        }
                        None => break,
                    };
                    front = new_front;
                    steps.push(item.into());
                }
                steps
            }
            (Some(t), None) => vec![ScheduledSubStep::new_boxed(t)],
            _ => unreachable!(),
        };

        Ok(Some(Self {
            sequence_number: self.sequence_number + 1,
            steps,
            status: StepStatus::Unsaturated,
            time,
            shared_step_state: Self::new_shared_step_state(),
            event_count: 0,
            can_produce_events: true,
            #[cfg(debug_assertions)]
            uuid_self: uuid::Uuid::new_v4(),
            #[cfg(debug_assertions)]
            uuid_prev: Some(self.uuid_self),
        }))
    }

    /// Create a new step that is ready to be saturated.
    ///
    /// This will only create a step from a scheduled event, and should be used if you know there
    /// isn't another input event in the future.
    pub fn next_scheduled_unsaturated(&self) -> Result<Option<Self>, NextUnsaturatedErr>
    where
        T: Clone,
    {
        self.next_unsaturated(&mut None)
    }

    /// Begin saturating the step by taking the transposer and metadata from the previous step.
    ///
    /// This moves the previous step from Saturated to Unsaturated, and the current step from Unsaturated to Saturating.
    ///
    /// # Errors
    ///
    /// - If the previous step is not Saturated.
    /// - If the current step is not Unsaturated.
    /// - If the previous step's UUID does not match the current step's UUID. (only when debug assertions are enabled)
    pub fn start_saturate_take(&mut self, prev: &mut Self) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != Some(prev.uuid_self) {
            return Err(SaturateErr::IncorrectPrevious);
        }

        self.start_saturate(prev.take()?)
    }

    /// Begin saturating the step by cloning the transposer and metadata from the previous step.
    ///
    /// This moves the current step from Unsaturated to Saturating, without changing the previous step.
    ///
    /// # Errors
    ///
    /// - If the previous step is not Saturated.
    /// - If the current step is not Unsaturated.
    /// - If the previous step's UUID does not match the current step's UUID. (only when debug assertions are enabled)
    pub fn start_saturate_clone(&mut self, prev: &Self) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        #[cfg(debug_assertions)]
        if self.uuid_prev != Some(prev.uuid_self) {
            return Err(SaturateErr::IncorrectPrevious);
        }

        self.start_saturate(prev.clone()?)
    }

    fn take(&mut self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr> {
        match self.get_step_status_mut() {
            ActiveStepStatusMut::Saturated(step) => {
                Ok(step.as_mut().take_finished_transposer().unwrap())
            }
            _ => Err(SaturateErr::PreviousNotSaturated),
        }
    }

    fn clone(&self) -> Result<SharedPointer<WrappedTransposer<T, P>, P>, SaturateErr> {
        match self.get_step_status_ref() {
            ActiveStepStatusRef::Saturated(t) => Ok(SharedPointer::clone(t)),
            _ => Err(SaturateErr::PreviousNotSaturated),
        }
    }

    fn start_saturate(
        &mut self,
        wrapped_transposer: SharedPointer<WrappedTransposer<T, P>, P>,
    ) -> Result<(), SaturateErr>
    where
        T: Clone,
    {
        *self.get_output_state_mut() = OutputEventManager::new_with_swallow_count(self.event_count);
        let shared_step_state = self.shared_step_state;
        let first = match self.get_step_status_mut() {
            ActiveStepStatusMut::Unsaturated(first) => first,
            _ => return Err(SaturateErr::SelfNotUnsaturated),
        };

        first
            .as_mut()
            .start_saturate(SharedPointer::clone(&wrapped_transposer), shared_step_state)
            .map_err(|e| match e {
                StartSaturateErr::SubStepTimeIsPast => panic!(),
                StartSaturateErr::NotUnsaturated => SaturateErr::SelfNotUnsaturated,
            })?;

        self.status = StepStatus::Saturating(0);
        Ok(())
    }

    /// Desaturate the step.
    ///
    /// This will move the step from Saturated or Saturating to Unsaturated, and all sub steps will be desaturated.
    ///
    /// When you desaturate a step, subsequent saturations *WILL NOT* result in re-emissions of the events that
    /// were emitted during the previous saturation, even if the previous saturation never completed. Think of this
    /// as the step remembering the events that were emitted, and skipping them if they are emmitted when the step
    /// is re-saturated. This will not prevent identical events from being emitted, however. The only observable
    /// difference (from the perspective of the transposer) is that the `emit_event` futures will immediately return
    /// `Poll::Ready` if the event was emitted during the previous saturation.
    ///
    /// This also resets the stored input state
    pub fn desaturate(&mut self) {
        match self.get_step_status_mut() {
            ActiveStepStatusMut::Saturated(step) => step.as_mut().desaturate(),
            ActiveStepStatusMut::Saturating(step) => step.as_mut().desaturate(),
            _ => {}
        }

        Self::drop_shared_step_state(self.shared_step_state);
        self.shared_step_state = Self::new_shared_step_state();
        self.status = StepStatus::Unsaturated;
    }

    /// Poll a saturated step toward completion.
    ///
    /// While this resembles a future, it is not a future, and has more types of results.
    ///
    /// # Returns
    ///
    /// - If the step is ready, this will move the step from Saturating to Saturated, and return `Ok(StepPoll::Ready)`.
    /// - If the step is not ready:
    ///     - If the step has emitted an event, and is waiting for the event to be extracted, this will return `Ok(StepPoll::Emitted(event))`.
    ///     - If the step has requested an input state and is waiting for it to be provided, this will return `Ok(StepPoll::StateRequested(type_id))`.
    ///
    /// # Errors
    ///
    /// - If the step is unsaturated, this will return `Err(PollErr::Unsaturated)`.
    /// - If the step is saturated, this will return `Err(PollErr::Saturated)`.
    pub fn poll(&mut self, waker: &Waker) -> Result<StepPoll<T>, PollErr>
    where
        T: Clone,
    {
        loop {
            let time = self.get_time();
            let step_count = self.steps.len();
            let current_index = match self.status {
                StepStatus::Saturating(i) => i,
                _ => return Err(PollErr::Unsaturated),
            };
            let mut sub_step = match self.get_step_status_mut() {
                ActiveStepStatusMut::Saturating(step) => step.as_mut(),
                _ => unreachable!(),
            };

            match sub_step.as_mut().poll(waker)? {
                Poll::Pending => {
                    if let Some(output_event) = self.get_output_state_mut().try_take_value() {
                        self.event_count += 1;
                        break Ok(StepPoll::Emitted(output_event));
                    }

                    if let Some(erased_input) = self.get_input_state_mut().try_accept_request()
                    {
                        break Ok(StepPoll::StateRequested(erased_input));
                    }

                    break Ok(StepPoll::Pending);
                }
                Poll::Ready(()) => {
                    // if we just finished saturating the last step
                    if current_index + 1 == step_count {
                        // check if there are any scheduled events at this time, and if so push a step to handle them.
                        if let Some(t) = sub_step
                            .get_finished_transposer()
                            .unwrap()
                            .metadata
                            .get_next_scheduled_time()
                        {
                            let t_time = t.time;
                            if t_time == time {
                                self.steps.push(ScheduledSubStep::new_boxed(t_time));
                                self.status = StepStatus::Saturating(current_index + 1);
                                continue;
                            }
                        }

                        // if there are no scheduled events, we are done.
                        self.status = StepStatus::Saturated;
                        self.can_produce_events = false;
                        break Ok(StepPoll::Ready);
                    } else {
                        // advance to the next step.
                        let wrapped_transposer = sub_step.take_finished_transposer().unwrap();
                        let shared_step_state = self.shared_step_state;
                        let next_sub_step = self.steps.get_mut(current_index + 1).unwrap();
                        next_sub_step
                            .as_mut()
                            .start_saturate(wrapped_transposer, shared_step_state)
                            .unwrap();
                        self.status = StepStatus::Saturating(current_index + 1);
                        continue;
                    }
                }
            }
        }
    }

    /// Provide the input state that was requested by the step during polling.
    ///
    /// This will return `Ok(())` if the input state was successfully provided, and `Err(input_state)` if the
    /// input state was not requested, or if the input state was not of the correct type.
    pub fn provide_input_state(
        &mut self,
        erased_state: Box<ErasedInputState<T>>
    ) -> Result<(), Box<ErasedInputState<T>>> {
        self.get_input_state_mut().provide_input_state(erased_state)
    }

    /// Begin interpolating the outut state of the step to the given time.
    ///
    /// This will return an `Interpolation` object that can be used like a future. While this is a future,
    /// it must be polled manually since input state may need to be provided between polls.
    pub fn interpolate(&self, time: T::Time) -> Result<Interpolation<T, P>, InterpolateErr>
    where
        T: Clone,
    {
        let wrapped_transposer = match self.get_step_status_ref() {
            ActiveStepStatusRef::Saturated(wrapped_transposer) => wrapped_transposer.clone(),
            _ => return Err(InterpolateErr::NotSaturated),
        };

        #[cfg(debug_assertions)]
        if time < wrapped_transposer.metadata.last_updated.time {
            return Err(InterpolateErr::TimePast);
        }

        Ok(Interpolation::new(time, wrapped_transposer))
    }

    /// Discard the step, extracting and returning all input events, so they can be reused, perhaps with
    /// new events added, or some of the events removed.
    ///
    /// They will be emitted in sorted order (the order the transposer would see them).
    pub fn drain_inputs(mut self) -> impl IntoIterator<Item = BoxedInput<'a, T, P>> {
        // need to desaturate before dropping self, since saturating steps may point to shared state.
        for step in &mut self.steps {
            step.as_mut().desaturate();
        }

        let steps = core::mem::take(&mut self.steps);

        steps.into_iter().filter_map(|step| step.try_into().ok())
    }

    /// Get the time of the step.
    pub fn get_time(&self) -> T::Time {
        self.time
    }

    /// true if the step is unsaturated.
    pub fn is_unsaturated(&self) -> bool {
        matches!(self.status, StepStatus::Unsaturated)
    }

    /// true if the step is saturating.
    pub fn is_saturating(&self) -> bool {
        matches!(self.status, StepStatus::Saturating(_))
    }

    /// true if the step is saturated.
    pub fn is_saturated(&self) -> bool {
        matches!(self.status, StepStatus::Saturated)
    }

    /// true if the step might still produce events.
    ///
    /// generally this will only be false if the step has ever been fully saturated.
    pub fn can_produce_events(&self) -> bool {
        self.can_produce_events
    }
}

impl<'a, T: Transposer, P: SharedPointerKind> Ord for Step<'a, T, P> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.sequence_number.cmp(&other.sequence_number)
    }
}

impl<'a, T: Transposer, P: SharedPointerKind> PartialOrd for Step<'a, T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, T: Transposer, P: SharedPointerKind> PartialEq for Step<'a, T, P> {
    fn eq(&self, other: &Self) -> bool {
        self.sequence_number == other.sequence_number
    }
}

impl<'a, T: Transposer, P: SharedPointerKind> Eq for Step<'a, T, P> {}

/// The result of polling a step.
#[derive(PartialEq, Eq)]
pub enum StepPoll<T: Transposer> {
    /// The step has emitted an event. The waker may never be called, and the caller is responsible for
    /// calling `poll` again after handling the event.
    Emitted(T::OutputEvent),

    /// The step has requested an input state. The waker may never be called, and the caller is responsible for
    /// calling `poll` again after providing the requested input state.
    ///
    /// the type id is the type id of the input that was requested.
    ///
    /// the specific input can be retrieved by calling `get_requested_input` on the step, then provided by calling
    /// `provide_input_state` on the step.
    StateRequested(Box<ErasedInput<T>>),

    /// The step is still pending. The waker will be called when the step is ready to be polled again.
    Pending,

    /// The step is now saturated.
    Ready,
}

/// The error result of polling a step.
#[derive(Debug, PartialEq, Eq)]
pub enum PollErr {
    /// The step is unsaturated.
    Unsaturated,

    /// The step is saturated.
    Saturated,
}

/// The error result of interpolating a step.
#[derive(Debug)]
pub enum InterpolateErr {
    /// The step is not saturated.
    NotSaturated,

    /// The time to interpolate to is in the past.
    ///
    /// This is only available when debug assertions are enabled.
    #[cfg(debug_assertions)]
    TimePast,
}

/// The error result of getting the next unsaturated step.
#[derive(Debug)]
pub enum NextUnsaturatedErr {
    /// The step is not saturated.
    NotSaturated,

    /// The input event is in the past or present.
    ///
    /// This is only available when debug assertions are enabled.
    #[cfg(debug_assertions)]
    InputPastOrPresent,
}

/// The error result of starting to saturate a step.
#[derive(Debug)]
pub enum SaturateErr {
    /// The previous step is not saturated.
    PreviousNotSaturated,

    /// The current step is not unsaturated.
    SelfNotUnsaturated,

    /// The previous step's UUID does not match the current step's UUID.
    ///
    /// This is only available when debug assertions are enabled.
    #[cfg(debug_assertions)]
    IncorrectPrevious,
}
```


With these three pieces, how can i make a valid source which takes in some sources matching the transposer inputs and a transposer, and implements the Source trait?

i believe it needs to keep track of:

- the list of all steps since the earliest finalize of the inputs
- the lastest finalize time of each input
- which state wakers to call when each active interpolation may progress
- which state wakers to call when a previously saturated step may progress towards re-saturation
- the last interrupt waker
- whether or not each input had woken the interrupt waker
- which steps pulled state from which inputs
- active interpolations
- each channel that returned pending and why it returned pending

what do you think of this approach?