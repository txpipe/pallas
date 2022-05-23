use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Clone, Copy)]
pub struct SystemStart {
    pub start: SystemTime,
}

#[derive(Clone, Copy)]
pub struct RelativeTime {
    pub relative: Duration,
}

#[derive(Clone, Copy)]
pub struct SlotLength {
    pub get_slot_length: Duration,
}

// pub fn slotFromUTCTime()
pub fn to_relative_time(
    start_time: SystemStart,
    time: SystemTime,
) -> Result<Duration, SystemTimeError> {
    time.duration_since(start_time.start)
}
