use std::collections::HashSet;
use std::{borrow::Borrow, cell::UnsafeCell};

use std::hash::Hash;

use archery::ArcTK;

use crate::{
    source::{source_poll::TrySourcePoll, traits::SourceContext, Source, SourcePoll},
    transposer::{
        input_erasure::{ErasedInput, ErasedInputState, HasErasedInput, HasInput},
        step::BoxedInput,
        Transposer, TransposerInput,
    },
};

struct ErasedInputSourceImpl<I, Src> {
    input: I,
    source: Src,
}

impl<I: TransposerInput, Src> HasInput<I::Base> for ErasedInputSourceImpl<I, Src> {
    type Input = I;

    fn get_input(&self) -> &Self::Input {
        &self.input
    }
}

impl<I, Src> Source for ErasedInputSourceImpl<I, Src>
where
    I: TransposerInput,
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
    I::Base: Clone,
{
    type Time = Src::Time;
    type Event = BoxedInput<'static, I::Base, ArcTK>;
    type State = Box<ErasedInputState<I::Base>>;

    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        match self.source.poll(time, cx)? {
            SourcePoll::Ready {
                state,
                next_event_at,
            } => {
                let state = ErasedInputState::new(self.input, state);
                Ok(SourcePoll::Ready {
                    state,
                    next_event_at,
                })
            }
            SourcePoll::Interrupt { time, interrupt } => {
                let interrupt = interrupt.map_event(|e| BoxedInput::new(time, self.input, e));
                Ok(SourcePoll::Interrupt { time, interrupt })
            }
            SourcePoll::Pending => Ok(SourcePoll::Pending),
        }
    }

    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        match self.source.poll_forget(time, cx)? {
            SourcePoll::Ready {
                state,
                next_event_at,
            } => {
                let state = ErasedInputState::new(self.input, state);
                Ok(SourcePoll::Ready {
                    state,
                    next_event_at,
                })
            }
            SourcePoll::Interrupt { time, interrupt } => {
                let interrupt = interrupt.map_event(|e| BoxedInput::new(time, self.input, e));
                Ok(SourcePoll::Interrupt { time, interrupt })
            }
            SourcePoll::Pending => Ok(SourcePoll::Pending),
        }
    }

    fn poll_events(
        &mut self,
        time: Self::Time,
        all_channel_waker: std::task::Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        match self.source.poll_events(time, all_channel_waker)? {
            SourcePoll::Ready {
                state,
                next_event_at,
            } => Ok(SourcePoll::Ready {
                state,
                next_event_at,
            }),
            SourcePoll::Interrupt { time, interrupt } => {
                let interrupt = interrupt.map_event(|e| BoxedInput::new(time, self.input, e));
                Ok(SourcePoll::Interrupt { time, interrupt })
            }
            SourcePoll::Pending => Ok(SourcePoll::Pending),
        }
    }

    fn release_channel(&mut self, channel: usize) {
        self.source.release_channel(channel)
    }

    fn advance(&mut self, time: Self::Time) {
        self.source.advance(time)
    }

    fn max_channel(&mut self) -> std::num::NonZeroUsize {
        self.source.max_channel()
    }
}

trait ErasedSourceTrait<T: Transposer + 'static>:
    Source<Time = T::Time, Event = BoxedInput<'static, T, ArcTK>, State = Box<ErasedInputState<T>>>
    + HasErasedInput<T>
{
}

impl<I, Src> ErasedSourceTrait<I::Base> for ErasedInputSourceImpl<I, Src>
where
    I: TransposerInput,
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
    I::Base: Clone,
{
}

#[repr(transparent)]
pub struct ErasedInputSource<T: Transposer + 'static>(UnsafeCell<Box<dyn ErasedSourceTrait<T>>>);

impl<T: Transposer + 'static> ErasedInputSource<T> {
    pub fn new<I, Src>(input: I, source: Src) -> Self
    where
        T: Clone,
        I: TransposerInput<Base = T>,
        Src: Source<
                Time = <I::Base as Transposer>::Time,
                Event = I::InputEvent,
                State = I::InputState,
            > + 'static,
    {
        let inner = ErasedInputSourceImpl { input, source };
        let inner: Box<dyn ErasedSourceTrait<T>> = Box::new(inner);
        let inner = UnsafeCell::new(inner);
        Self(inner)
    }

    pub unsafe fn get_src_mut(
        &self,
    ) -> &mut dyn Source<
        Time = T::Time,
        Event = BoxedInput<'static, T, ArcTK>,
        State = Box<ErasedInputState<T>>,
    > {
        unsafe { &mut *self.0.get() }.as_mut()
    }
}

impl<T: Transposer + 'static> Source for ErasedInputSource<T> {
    type Time = T::Time;
    type Event = BoxedInput<'static, T, ArcTK>;
    type State = Box<ErasedInputState<T>>;

    fn poll(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        self.0.get_mut().poll(time, cx)
    }

    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        self.0.get_mut().poll_forget(time, cx)
    }

    fn poll_events(
        &mut self,
        time: Self::Time,
        all_channel_waker: std::task::Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        self.0.get_mut().poll_events(time, all_channel_waker)
    }

    fn release_channel(&mut self, channel: usize) {
        self.0.get_mut().release_channel(channel)
    }

    fn advance(&mut self, time: Self::Time) {
        self.0.get_mut().advance(time)
    }

    fn max_channel(&mut self) -> std::num::NonZeroUsize {
        self.0.get_mut().max_channel()
    }
}

impl<T: Transposer + 'static> Hash for ErasedInputSource<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe { (*self.0.get()).as_ref() }.get_input_type_value_hash(state);
    }
}

impl<T: Transposer + 'static> PartialEq for ErasedInputSource<T> {
    fn eq(&self, other: &Self) -> bool {
        let (s, o) = unsafe { ((*self.0.get()).as_ref(), (*other.0.get()).as_ref()) };
        s.inputs_eq(o)
    }
}

impl<T: Transposer + 'static> Eq for ErasedInputSource<T> {}

impl<T: Transposer + 'static> Borrow<ErasedInput<T>> for ErasedInputSource<T> {
    fn borrow(&self) -> &ErasedInput<T> {
        let inner_ref = unsafe { (*self.0.get()).as_ref() };
        let inner_ref_casted: &dyn HasErasedInput<T> = inner_ref;
        inner_ref_casted.into()
    }
}

pub struct ErasedInputSourceCollection<T: Transposer + 'static>(HashSet<ErasedInputSource<T>>);

impl<T: Transposer + 'static> ErasedInputSourceCollection<T> {
    pub fn new(inputs: HashSet<ErasedInputSource<T>>) -> Self {
        Self(inputs)
    }

    pub fn get_source_mut(
        &mut self,
        input: &ErasedInput<T>,
    ) -> Option<
        &mut dyn Source<
            Time = T::Time,
            Event = BoxedInput<'static, T, ArcTK>,
            State = Box<ErasedInputState<T>>,
        >,
    > {
        match self.0.get(input) {
            Some(source) => Some(unsafe { source.get_src_mut() }),
            None => None,
        }
    }
}
