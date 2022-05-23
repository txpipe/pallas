use std::{time::{Duration, SystemTime}, thread::sleep};

use crate::wall_clock::{RelativeTime, SlotLength, SystemStart, to_relative_time};

#[derive(Debug)]
pub struct BlockChainTime {
    pub current_slot: CurrentSlot,
}

#[derive(Debug)]
pub struct CurrentSlot {
    pub slot: Option<u64>,
}

// Stub
pub fn simple_blockchain_time(
    time: SystemTime,
    slot_length: SlotLength,
    max_clock_rewind: Duration,
) -> BlockChainTime {
    let current_slot = CurrentSlot { slot: None };
    BlockChainTime { current_slot }
}

// Stub
pub fn slot_from_utc_time(slot_length: SlotLength, relative_time: RelativeTime) -> (u64, Duration) {
    (0, Duration::new(0, 0))
}

pub fn delay_until_next_slot(slot_length: SlotLength, now: RelativeTime) -> Duration {
    let (_, time_spent) = slot_from_utc_time(slot_length, now);
    slot_length.get_slot_length - time_spent
}

// TODO: Has side effect of thread delay then returns new current slot
// Is there better behavior in rust?
pub fn wait_until_next_slot(
    start: SystemStart,
    time: SystemTime,
    slot_length: SlotLength,
    max_clock_rewind: Duration,
    current_slot: CurrentSlot,
) -> CurrentSlot {
    let current_relative_time = RelativeTime {relative: to_relative_time(start, time).unwrap()};
    let delay = delay_until_next_slot(slot_length, current_relative_time);
    sleep(delay);
    let new_relative_time = RelativeTime{relative: to_relative_time(start, SystemTime::now()).unwrap()}
    let (new_current_slot, _) = slot_from_utc_time(slot_length, new_relative_time);

    if new_current_slot > current_slot.slot.unwrap() {
        CurrentSlot { slot: Some(new_current_slot) }
    }
    else if new_current_slot <= current_slot.slot.unwrap() && 
            current_relative_time.relative.checked_sub(new_relative_time.relative).unwrap() <= max_clock_rewind {
                wait_until_next_slot(start, time, slot_length, max_clock_rewind, current_slot)
    }
    else{
        todo!("Throw Clock Moved Back Exception here")
    }
}


// TODO: Slot Watcher to implement
