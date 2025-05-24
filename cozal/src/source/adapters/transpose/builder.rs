use std::{
    collections::{BTreeSet, HashSet},
    num::NonZeroUsize,
};

use hashbrown::HashMap;

use crate::{
    source::{
        Source,
        source_poll::{LowerBound, UpperBound},
    },
    transposer::{
        Transposer, TransposerInput, TransposerInputEventHandler, input_erasure::ErasedInput,
        step::PreInitStep,
    },
};

use super::{
    erased_input_source_collection::{ErasedInputSource, ErasedInputSourceCollection}, input_channel_reservations::InputChannelReservations, input_source_collection::InputSourceCollection, transpose_interrupt_waker::TransposeWakerObserver, working_timeline_slice::WorkingTimelineSlice, Transpose, TransposeMain
};

pub struct TransposeBuilder<T: Transposer + 'static> {
    transposer: T,
    pre_init_step: PreInitStep<T>,
    input_sources: HashSet<ErasedInputSource<T>>,
    rng_seed: [u8; 32],
    max_channels: NonZeroUsize,
}

impl<T: Transposer + Clone + 'static> TransposeBuilder<T> {
    /// Create a new builder
    pub fn new(transposer: T, rng_seed: [u8; 32], max_channels: NonZeroUsize) -> Self {
        Self {
            transposer,
            pre_init_step: PreInitStep::new(),
            input_sources: HashSet::new(),
            rng_seed,
            max_channels,
        }
    }

    /// Assign an input source.
    ///
    /// Returns the self for chaining.
    pub fn add_input<I, S>(mut self, input: I, source: S) -> Result<Self, (I, S)>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
        S: 'static + Source<Time = T::Time, Event = I::InputEvent, State = I::InputState>,
    {
        let erased_input = ErasedInput::new(input);
        if self.input_sources.contains(&*erased_input) {
            return Err((input, source));
        }

        self.pre_init_step.add_input(input);
        if source.max_channel() < self.max_channels {
            todo!()
            // this should multiplex the source up to the desired max_channels value.
            // self.input_sources
            //     .insert()
        } else {
            self.input_sources
                .insert(ErasedInputSource::new(input, source));
        }

        Ok(self)
    }

    /// Assign an input source.
    ///
    /// Returns the reference for chaining.
    pub fn add_input_mut<I, S>(&mut self, input: I, source: S) -> Result<&mut Self, (I, S)>
    where
        I: TransposerInput<Base = T>,
        T: TransposerInputEventHandler<I>,
        S: 'static + Source<Time = T::Time, Event = I::InputEvent, State = I::InputState>,
    {
        let erased_input = ErasedInput::new(input);
        if self.input_sources.contains(&*erased_input) {
            return Err((input, source));
        }

        self.pre_init_step.add_input(input);
        if source.max_channel() < self.max_channels {
            todo!()
            // this should multiplex the source up to the desired max_channels value.
            // self.input_sources
            //     .insert()
        } else {
            self.input_sources
                .insert(ErasedInputSource::new(input, source));
        }

        Ok(self)
    }

    /// Complete the build operation.
    pub fn build(self) -> Result<Transpose<T>, ()> {
        let Self {
            transposer,
            pre_init_step,
            rng_seed,
            input_sources,
            max_channels: _,
        } = self;

        let working_timeline_slice = WorkingTimelineSlice::new(transposer, pre_init_step, rng_seed).map_err(|_| ())?;

        let input_sources = ErasedInputSourceCollection::new(input_sources)?;
        let wakers = TransposeWakerObserver::new(input_sources.iter_with_hashes().map(|(h, ..)| h));
        let input_sources = InputSourceCollection::new(input_sources);

        Ok(Transpose {
            main: TransposeMain {
                input_sources,
                working_timeline_slice,
                // interpolations: HashMap::new(),
                // next_interpolation_uuid: 0,
                // channel_reservations: InputChannelReservations::new(),
                // advance_upper_bound: UpperBound::min(),
                // advance_lower_bound: LowerBound::min(),
                // last_emitted_finalize: LowerBound::min(),
                // returned_state_times: BTreeSet::new(),
            },
            wakers,
        })
    }
}
