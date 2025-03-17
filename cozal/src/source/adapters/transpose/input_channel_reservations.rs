use std::collections::{
    BTreeSet,
    btree_set::{Entry, VacantEntry},
};

pub struct InputChannelReservations {
    pub input_channels: BTreeSet<InputChannelReservation>,
}

#[derive(Hash, Ord, PartialEq, PartialOrd, Eq, Clone)]
pub struct InputChannelReservation {
    pub input_hash: u64,
    pub input_channel: usize,
}

impl InputChannelReservations {
    pub fn new() -> Self {
        Self {
            input_channels: BTreeSet::new(),
        }
    }

    /// Find the first available channel for the specified input.
    ///
    /// This returns a VacantEntry, which can be used to perform the insertion.
    pub fn get_first_available_channel(
        &mut self,
        input_hash: u64,
    ) -> VacantEntry<InputChannelReservation> {
        let start_key = InputChannelReservation {
            input_hash,
            input_channel: 0,
        };
        let end_key = InputChannelReservation {
            input_hash,
            input_channel: usize::MAX,
        };
        let items = self.input_channels.range(start_key..=end_key);

        let mut channel = 0;
        for item in items {
            if channel != item.input_channel {
                break;
            }
            channel += 1;
        }

        let key = InputChannelReservation {
            input_hash,
            input_channel: channel,
        };

        match self.input_channels.entry(key) {
            Entry::Occupied(_) => panic!(),
            Entry::Vacant(vacant_entry) => vacant_entry,
        }
    }

    pub fn clear_channel(&mut self, input_hash: u64, input_channel: usize) {
        self.input_channels.remove(&InputChannelReservation {
            input_hash,
            input_channel,
        });
    }
}
