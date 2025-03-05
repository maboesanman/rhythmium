use std::borrow::Borrow;
use std::collections::HashSet;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroUsize;

use crate::source::source_poll::Interrupt;
use crate::transposer::TransposerInputEventHandler;
use crate::{
    source::{source_poll::TrySourcePoll, traits::SourceContext, Source, SourcePoll},
    transposer::{
        input_erasure::{ErasedInput, ErasedInputState, HasErasedInput, HasInput},
        step::BoxedInput,
        Transposer, TransposerInput,
    },
};
use archery::ArcTK;
use hashbrown::HashTable;

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

impl<I, Src> ErasedInputSourceImpl<I, Src>
where
    I: TransposerInput,
    Src: Source<Time = <I::Base as Transposer>::Time, Event = I::InputEvent, State = I::InputState>,
    I::Base: Clone,
{
    fn resolve_interrupts(
        &self,
        time: <I::Base as Transposer>::Time,
        interrupt: Interrupt<I::InputEvent>,
    ) -> Option<Interrupt<BoxedInput<'static, I::Base, ArcTK>>> {
        match interrupt {
            Interrupt::Event(event) => {
                if !<I::Base as TransposerInputEventHandler<I>>::can_handle(time, &event) {
                    None
                } else {
                    Some(Interrupt::Event(BoxedInput::new(time, self.input, event)))
                }
            }
            Interrupt::FinalizedEvent(event) => {
                if !<I::Base as TransposerInputEventHandler<I>>::can_handle(time, &event) {
                    Some(Interrupt::Finalize)
                } else {
                    Some(Interrupt::Event(BoxedInput::new(time, self.input, event)))
                }
            }
            Interrupt::Rollback => Some(Interrupt::Rollback),
            Interrupt::Finalize => Some(Interrupt::Finalize),
            Interrupt::Complete => Some(Interrupt::Complete),
        }
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
        loop {
            break match self.source.poll(time, cx.clone())? {
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
                    let interrupt = match self.resolve_interrupts(time, interrupt) {
                        Some(i) => i,
                        None => continue,
                    };
                    Ok(SourcePoll::Interrupt { time, interrupt })
                }
                SourcePoll::Pending => Ok(SourcePoll::Pending),
            };
        }
    }

    fn poll_forget(
        &mut self,
        time: Self::Time,
        cx: SourceContext,
    ) -> TrySourcePoll<Self::Time, Self::Event, Self::State> {
        loop {
            break match self.source.poll_forget(time, cx.clone())? {
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
                    let interrupt = match self.resolve_interrupts(time, interrupt) {
                        Some(i) => i,
                        None => continue,
                    };
                    Ok(SourcePoll::Interrupt { time, interrupt })
                }
                SourcePoll::Pending => Ok(SourcePoll::Pending),
            };
        }
    }

    fn poll_events(
        &mut self,
        time: Self::Time,
        interrupt_waker: std::task::Waker,
    ) -> TrySourcePoll<Self::Time, Self::Event, ()> {
        loop {
            break match self.source.poll_events(time, interrupt_waker.clone())? {
                SourcePoll::Ready {
                    state,
                    next_event_at,
                } => Ok(SourcePoll::Ready {
                    state,
                    next_event_at,
                }),
                SourcePoll::Interrupt { time, interrupt } => {
                    let interrupt = match self.resolve_interrupts(time, interrupt) {
                        Some(i) => i,
                        None => continue,
                    };
                    Ok(SourcePoll::Interrupt { time, interrupt })
                }
                SourcePoll::Pending => Ok(SourcePoll::Pending),
            };
        }
    }

    fn release_channel(&mut self, channel: usize) {
        self.source.release_channel(channel)
    }

    fn advance(&mut self, time: Self::Time) {
        self.source.advance(time)
    }

    fn advance_final(&mut self) {
        self.source.advance_final()
    }

    fn max_channel(&self) -> std::num::NonZeroUsize {
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

pub struct ErasedInputSource<T: Transposer + 'static>(Box<dyn ErasedSourceTrait<T>>);

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
        // let inner = UnsafeCell::new(inner);
        Self(inner)
    }

    pub fn max_channel(&self) -> NonZeroUsize {
        self.0.max_channel()
    }
}

impl<T: Transposer + 'static> Hash for ErasedInputSource<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ref().get_input_type_value_hash(state);
    }
}

impl<T: Transposer + 'static> PartialEq for ErasedInputSource<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref().inputs_eq(other.0.as_ref())
    }
}

impl<T: Transposer + 'static> Eq for ErasedInputSource<T> {}

impl<T: Transposer + 'static> Borrow<ErasedInput<T>> for ErasedInputSource<T> {
    fn borrow(&self) -> &ErasedInput<T> {
        let inner_ref = self.0.as_ref();
        let inner_ref_casted: &dyn HasErasedInput<T> = inner_ref;
        inner_ref_casted.into()
    }
}

pub struct ErasedInputSourceCollection<T: Transposer + 'static, M>(HashTable<TableEntry<T, M>>);

struct TableEntry<T: Transposer + 'static, M> {
    hash: u64,
    input: ErasedInputSource<T>,
    metadata: M,
}

impl<T: Transposer + 'static, M: Default> TableEntry<T, M> {
    fn new(input: ErasedInputSource<T>) -> Self {
        let hash = ErasedInputSourceCollection::<T, M>::hash(input.borrow());
        let metadata = M::default();

        Self {
            hash,
            input,
            metadata,
        }
    }
}

impl<T: Transposer + 'static, M> TableEntry<T, M> {
    fn into_guard(&mut self) -> ErasedInputSourceGuard<T, M> {
        ErasedInputSourceGuard {
            source: &mut self.input,
            metadata: &mut self.metadata,
        }
    }
}

pub struct ErasedInputSourceGuard<'a, T: Transposer + 'static, M> {
    // this is actually a mutable reference with some restrictions (we can't do anything that would change the input key)
    source: &'a mut ErasedInputSource<T>,
    metadata: &'a mut M,
}

impl<T: Transposer + 'static, M: Default> ErasedInputSourceCollection<T, M> {
    pub fn new(inputs: HashSet<ErasedInputSource<T>>) -> Result<Self, ()> {
        let mut inner = HashTable::new();
        for (hash, input) in inputs.into_iter().map(|i| (Self::hash(i.borrow()), i)) {
            inner.insert_unique(hash, TableEntry::new(input), Self::hasher);
        }

        Ok(Self(inner))
    }
}

impl<T: Transposer + 'static, M> ErasedInputSourceCollection<T, M> {
    fn hash(input: &ErasedInput<T>) -> u64 {
        let mut s = DefaultHasher::new();
        input.hash(&mut s);
        s.finish()
    }

    fn hasher(item: &TableEntry<T, M>) -> u64 {
        item.hash
    }

    pub fn get_input_by_hash(&mut self, input_hash: u64) -> Option<ErasedInputSourceGuard<T, M>> {
        self.0
            .find_mut(input_hash, |entry| entry.hash == input_hash)
            .map(TableEntry::into_guard)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ErasedInputSource<T>, &M)> {
        self.0.iter().map(|entry| (&entry.input, &entry.metadata))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = ErasedInputSourceGuard<T, M>> {
        self.0.iter_mut().map(TableEntry::into_guard)
    }

    pub fn iter_with_hashes(&self) -> impl Iterator<Item = (u64, &ErasedInputSource<T>, &M)> {
        self.0
            .iter()
            .map(|entry| (entry.hash, &entry.input, &entry.metadata))
    }
}

impl<T: Transposer + 'static, M> ErasedInputSourceGuard<'_, T, M> {
    pub fn get_source(
        &self,
    ) -> &dyn Source<
        Time = T::Time,
        Event = BoxedInput<'static, T, ArcTK>,
        State = Box<ErasedInputState<T>>,
    > {
        self.source.0.as_ref()
    }

    pub fn get_source_mut(
        &mut self,
    ) -> &mut dyn Source<
        Time = T::Time,
        Event = BoxedInput<'static, T, ArcTK>,
        State = Box<ErasedInputState<T>>,
    > {
        self.source.0.as_mut()
    }

    pub fn get_metadata_mut(&mut self) -> &mut M {
        self.metadata
    }
}
