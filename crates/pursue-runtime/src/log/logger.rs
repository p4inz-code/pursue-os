//! The logger: level filtering, component scoping, and a process-global
//! instance.

use pursue_core::{Error, Result};
use serde_json::Value as JsonValue;
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::Level;
use super::record::Record;
use super::sink::{Sink, StderrSink};

/// The default component name used by [`Logger::default`] and the builder.
pub const DEFAULT_COMPONENT: &str = "pursue";

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A leveled, component-scoped structured logger.
///
/// Records below the configured level are discarded before any sink is
/// touched. Sink errors are intentionally best-effort: logging must not
/// disrupt the process, so a failing sink is silently ignored. This is
/// documented behavior for the foundation.
#[derive(Clone)]
pub struct Logger {
    level: Level,
    component: Arc<str>,
    sink: Arc<dyn Sink>,
}

impl Logger {
    /// Creates a logger with a fixed level, component, and sink.
    ///
    /// Fails if `component` is empty or whitespace-only.
    pub fn new(level: Level, component: &str, sink: Arc<dyn Sink>) -> Result<Self> {
        let component = component.trim();
        if component.is_empty() {
            return Err(Error::InvalidInput(
                "log component must not be empty".into(),
            ));
        }
        Ok(Self {
            level,
            component: Arc::from(component),
            sink,
        })
    }

    /// Starts a builder with sane defaults.
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }

    /// The level below which records are discarded.
    pub fn level(&self) -> Level {
        self.level
    }

    /// The component this logger writes records under.
    pub fn component(&self) -> &str {
        &self.component
    }

    /// Whether a record at `level` would be emitted.
    pub fn enabled(&self, level: Level) -> bool {
        level <= self.level
    }

    /// Emits a record with no structured fields.
    pub fn log(&self, level: Level, message: &str) {
        if !self.enabled(level) {
            return;
        }
        let record = Record::new(now_unix(), level, &self.component, message, Vec::new())
            .expect("component validated at construction");
        let _ = self.sink.write(&record);
    }

    /// Emits a record with structured `fields` (key/value pairs).
    ///
    /// The component is validated at construction, so record construction
    /// cannot fail here.
    pub fn log_fields(&self, level: Level, message: &str, fields: Vec<(String, JsonValue)>) {
        if !self.enabled(level) {
            return;
        }
        let record = Record::new(now_unix(), level, &self.component, message, fields)
            .expect("component validated at construction");
        let _ = self.sink.write(&record);
    }

    /// Emits an error-level record.
    pub fn error(&self, message: &str) {
        self.log(Level::Error, message);
    }

    /// Emits a warn-level record.
    pub fn warn(&self, message: &str) {
        self.log(Level::Warn, message);
    }

    /// Emits an info-level record.
    pub fn info(&self, message: &str) {
        self.log(Level::Info, message);
    }

    /// Emits a debug-level record.
    pub fn debug(&self, message: &str) {
        self.log(Level::Debug, message);
    }

    /// Emits a trace-level record.
    pub fn trace(&self, message: &str) {
        self.log(Level::Trace, message);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Logger::new(Level::Info, DEFAULT_COMPONENT, Arc::new(StderrSink::new()))
            .expect("default component is valid")
    }
}

/// Builder for [`Logger`].
#[derive(Clone)]
pub struct LoggerBuilder {
    level: Level,
    component: String,
    sink: Arc<dyn Sink>,
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self {
            level: Level::Info,
            component: DEFAULT_COMPONENT.to_string(),
            sink: Arc::new(StderrSink::new()),
        }
    }
}

impl LoggerBuilder {
    /// Starts with sane defaults (info level, `pursue` component, stderr).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum emitted level.
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Sets the component name.
    pub fn component(mut self, component: impl Into<String>) -> Self {
        self.component = component.into();
        self
    }

    /// Sets the output sink.
    pub fn sink(mut self, sink: Arc<dyn Sink>) -> Self {
        self.sink = sink;
        self
    }

    /// Builds the logger; fails if the component is empty.
    pub fn build(self) -> Result<Logger> {
        Logger::new(self.level, &self.component, self.sink)
    }
}

static GLOBAL_LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initializes the process-global logger.
///
/// Fails if the global logger is already initialized. Only one test in this
/// crate may exercise this function.
pub fn init_global(logger: Logger) -> Result<()> {
    GLOBAL_LOGGER
        .set(logger)
        .map_err(|_| Error::InvalidInput("global logger already initialized".into()))
}

/// The process-global logger, if initialized.
pub fn try_global() -> Option<&'static Logger> {
    GLOBAL_LOGGER.get()
}

/// The process-global logger.
///
/// Panics if [`init_global`] was never called.
pub fn global() -> &'static Logger {
    GLOBAL_LOGGER
        .get()
        .expect("global logger not initialized; call init_global first")
}

#[cfg(test)]
mod tests {
    use super::{Logger, LoggerBuilder, global, init_global, try_global};
    use crate::log::{Level, TestSink};
    use serde_json::json;
    use std::sync::Arc;

    fn test_logger(level: Level) -> (Logger, Arc<TestSink>) {
        let sink = Arc::new(TestSink::new());
        let logger = Logger::new(level, "test", Arc::clone(&sink) as Arc<dyn super::Sink>).unwrap();
        (logger, sink)
    }

    #[test]
    fn level_filtering_discards_below_threshold() {
        let (logger, sink) = test_logger(Level::Info);
        logger.error("e");
        logger.warn("w");
        logger.info("i");
        logger.debug("d");
        logger.trace("t");
        let records = sink.records();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].level(), Level::Error);
        assert_eq!(records[1].level(), Level::Warn);
        assert_eq!(records[2].level(), Level::Info);
        assert!(!logger.enabled(Level::Debug));
        assert!(logger.enabled(Level::Info));
    }

    #[test]
    fn structured_fields_are_captured() {
        let (logger, sink) = test_logger(Level::Debug);
        logger.log_fields(
            Level::Debug,
            "request handled",
            vec![
                ("service".to_string(), json!("evidence")),
                ("elapsed_ms".to_string(), json!(12)),
            ],
        );
        let record = &sink.records()[0];
        assert_eq!(record.fields().len(), 2);
        assert!(
            record
                .fields()
                .contains(&("service".to_string(), json!("evidence")))
        );
        assert_eq!(record.message(), "request handled");
    }

    #[test]
    fn component_is_scoped_and_validated() {
        let (logger, _sink) = test_logger(Level::Info);
        assert_eq!(logger.component(), "test");
        assert!(Logger::new(Level::Info, "  ", Arc::new(crate::log::NullSink)).is_err());
    }

    #[test]
    fn builder_produces_equivalent_logger() {
        let sink = Arc::new(TestSink::new());
        let logger = LoggerBuilder::new()
            .level(Level::Trace)
            .component("builder-test")
            .sink(Arc::clone(&sink) as Arc<dyn super::Sink>)
            .build()
            .unwrap();
        assert_eq!(logger.level(), Level::Trace);
        assert_eq!(logger.component(), "builder-test");
        logger.trace("visible");
        assert_eq!(sink.count(), 1);
    }

    #[test]
    fn logger_default_uses_stderr_and_default_component() {
        let logger = Logger::default();
        assert_eq!(logger.level(), Level::Info);
        assert_eq!(logger.component(), super::DEFAULT_COMPONENT);
    }

    #[test]
    fn global_logger_initialization() {
        assert!(try_global().is_none());
        let sink = Arc::new(TestSink::new());
        let logger = Logger::new(
            Level::Info,
            "global",
            Arc::clone(&sink) as Arc<dyn super::Sink>,
        )
        .unwrap();
        init_global(logger).unwrap();
        // Second initialization must fail.
        let second = Logger::new(Level::Info, "again", Arc::new(crate::log::NullSink)).unwrap();
        assert!(init_global(second).is_err());
        let global = global();
        assert_eq!(global.component(), "global");
        global.info("hello from global");
        assert_eq!(sink.count(), 1);
    }
}
