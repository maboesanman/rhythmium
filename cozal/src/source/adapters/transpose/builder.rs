use std::{
    collections::{BTreeSet, HashSet},
    num::NonZeroUsize,
};

use hashbrown::HashMap;

use crate::{
    source::Source,
    transposer::{
        input_erasure::ErasedInput, step::PreInitStep, Transposer, TransposerInput,
        TransposerInputEventHandler,
    },
};

use super::{
    erased_input_source_collection::{ErasedInputSource, ErasedInputSourceCollection},
    input_channel_reservations::InputChannelReservations,
    steps::StepList,
    transpose_interrupt_waker::TransposeWakerObserver,
    Transpose, TransposeMain,
};

pub struct TransposeBuilder<T: Transposer + 'static> {
    transposer: T,
    pre_init_step: PreInitStep<T>,
    input_sources: HashSet<ErasedInputSource<T>>,
    start_time: T::Time,
    rng_seed: [u8; 32],
    max_channels: NonZeroUsize,
}

impl<T: Transposer + Clone + 'static> TransposeBuilder<T> {
    /// Create a new builder
    pub fn new(
        transposer: T,
        start_time: T::Time,
        rng_seed: [u8; 32],
        max_channels: NonZeroUsize,
    ) -> Self {
        Self {
            transposer,
            pre_init_step: PreInitStep::new(),
            input_sources: HashSet::new(),
            start_time,
            rng_seed,
            max_channels,
        }
    }

    /// Assign an input source.
    ///
    /// Returns the reference for chaining.
    pub fn add_input<I, S>(&mut self, input: I, source: S) -> Result<&mut Self, (I, S)>
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
                .insert(ErasedInputSource::new(input, source, self.start_time));
        }

        Ok(self)
    }

    /// Complete the build operation.
    pub fn build(self) -> Result<Transpose<T>, ()> {
        let Self {
            transposer,
            pre_init_step,
            start_time,
            rng_seed,
            input_sources,
            max_channels: _,
        } = self;

        let steps =
            StepList::new(transposer, pre_init_step, start_time, rng_seed).map_err(|_| ())?;
            
        let input_sources = ErasedInputSourceCollection::new(input_sources)?;
        let wakers = TransposeWakerObserver::new(input_sources.iter_with_hashes().map(|(h, ..)| h));

        Ok(Transpose {
            main: TransposeMain {
                input_sources,
                steps,
                input_buffer: BTreeSet::new(),
                interpolations: HashMap::new(),
                next_interpolation_uuid: 0,
                wavefront_time: None,
                advance_time: None,
                channel_reservations: InputChannelReservations::new(),
                advance_final: false,
                complete: None,
                last_finalize: None,
                needs_signal: false,
            },
            wakers,
        })
    }
}
