use archery::ArcTK;

use crate::transposer::{step::Interpolation, Transposer};

use super::{erased_input_source_collection::ErasedInputSourceCollection, input_source_metadata::InputSourceMetaData};




pub struct ChannelUsage<T: Transposer> {
    pub status: ChannelStatus<T>,
    pub poll_time: T::Time,
    pub poll_type: PollType,
}

#[derive(Clone, Copy)]
pub enum PollType {
    Poll,
    PollForget,
    PollEvents
}

pub enum ChannelStatus<T: Transposer> {
    Step,
    StepInput {
        input_hash: u64,
        input_channel: u64,
    },
    Interpolation {
        interpolation: Interpolation<T, ArcTK>
    },
    InterpolationInput {
        interpolation: Interpolation<T, ArcTK>,
        input_hash: u64,
        input_channel: u64,
    },
}

impl<T: Transposer> ChannelUsage<T> {
    pub fn get_time(&self) -> T::Time {
        self.poll_time
    }

    pub fn get_poll_type(&self) -> PollType {
        self.poll_type
    }
}