//! Output sinks for log records.

use pursue_core::{Error, Result};
use serde_json::to_string;
use std::io::{self, Write};
use std::sync::Mutex;

use super::record::Record;

/// Destination for structured log records.
///
/// Implementations must be safe to share across threads (the default logger
/// clones its sink), so implementations needing interior mutability use a
/// `Mutex`.
pub trait Sink: Send + Sync {
    /// Writes one record. Errors are propagated to the caller.
    fn write(&self, record: &Record) -> Result<()>;

    /// Flushes any buffered output. Defaults to a no-op.
    fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// Writes records as one compact JSON object per line.
pub struct JsonLinesSink<W: Write + Send + Sync> {
    writer: Mutex<W>,
}

impl<W: Write + Send + Sync> JsonLinesSink<W> {
    /// Wraps `writer`.
    pub fn new(writer: W) -> Self {
        Self {
            writer: Mutex::new(writer),
        }
    }
}

impl<W: Write + Send + Sync> Sink for JsonLinesSink<W> {
    fn write(&self, record: &Record) -> Result<()> {
        let mut line = to_string(record)
            .map_err(|e| Error::InvalidInput(format!("log record serialization failed: {e}")))?;
        line.push('\n');
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        writer.write_all(line.as_bytes())?;
        Ok(())
    }

    fn flush(&self) -> Result<()> {
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        writer.flush()?;
        Ok(())
    }
}

/// A JSON-lines sink writing to standard error.
///
/// Standard error keeps log output separate from anything written to standard
/// output (e.g., by CLI tools or future services).
pub struct StderrSink(JsonLinesSink<io::Stderr>);

impl StderrSink {
    /// Creates a sink writing to the process standard error.
    pub fn new() -> Self {
        Self(JsonLinesSink::new(io::stderr()))
    }
}

impl Default for StderrSink {
    fn default() -> Self {
        Self::new()
    }
}

impl Sink for StderrSink {
    fn write(&self, record: &Record) -> Result<()> {
        self.0.write(record)
    }

    fn flush(&self) -> Result<()> {
        self.0.flush()
    }
}

/// A sink that discards every record.
pub struct NullSink;

impl Sink for NullSink {
    fn write(&self, _record: &Record) -> Result<()> {
        Ok(())
    }
}

/// A sink that captures records in memory for tests and diagnostics.
#[derive(Default)]
pub struct TestSink {
    records: Mutex<Vec<Record>>,
}

impl TestSink {
    /// Creates an empty capturing sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// All records written so far, in order.
    pub fn records(&self) -> Vec<Record> {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// The number of records captured so far.
    pub fn count(&self) -> usize {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// Removes all captured records.
    pub fn clear(&self) {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

impl Sink for TestSink {
    fn write(&self, record: &Record) -> Result<()> {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(record.clone());
        Ok(())
    }
}
