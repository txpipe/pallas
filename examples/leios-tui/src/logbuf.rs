//! A `tracing` layer that captures formatted log lines into a shared ring buffer
//! so they can be rendered inside the TUI's Log panel instead of being written to
//! stdout (which would corrupt the alternate screen).

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

/// Maximum number of log lines retained in the ring buffer.
const LOG_CAP: usize = 500;

/// A shared, bounded buffer of formatted log lines (newest at the back).
pub type SharedLog = Arc<Mutex<VecDeque<String>>>;

/// Creates an empty shared log buffer.
pub fn new_log() -> SharedLog {
    Arc::new(Mutex::new(VecDeque::with_capacity(LOG_CAP)))
}

/// Process start, used to stamp each log line with an uptime offset (avoids a
/// wall-clock dependency).
static START: OnceLock<Instant> = OnceLock::new();

fn uptime() -> String {
    let s = START.get_or_init(Instant::now).elapsed().as_secs();
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

/// A `tracing` layer that appends each event to a [`SharedLog`].
pub struct LogLayer {
    buf: SharedLog,
}

impl LogLayer {
    pub fn new(buf: SharedLog) -> Self {
        Self { buf }
    }
}

impl<S: Subscriber> Layer<S> for LogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MsgVisitor::default();
        event.record(&mut visitor);

        let meta = event.metadata();
        let line = format!(
            "{} {:>5} {}{}",
            uptime(),
            meta.level(),
            visitor.msg,
            visitor.fields
        );

        if let Ok(mut buf) = self.buf.lock() {
            buf.push_back(line);
            while buf.len() > LOG_CAP {
                buf.pop_front();
            }
        }
    }
}

/// Collects an event's `message` plus any `key=value` fields into strings.
#[derive(Default)]
struct MsgVisitor {
    msg: String,
    fields: String,
}

impl Visit for MsgVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.msg = format!("{value:?}");
        } else {
            let _ = write!(self.fields, " {}={:?}", field.name(), value);
        }
    }
}
