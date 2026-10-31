//! Rolling log files in `%LOCALAPPDATA%\nd104-sync\logs` (7 days).

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::EnvFilter;

/// Starts file logging. Keep the guard alive for the whole run.
pub fn init() -> Option<WorkerGuard> {
    let dir = nd104_core::config::log_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("nd104-sync")
        .filename_suffix("log")
        .max_log_files(7)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "info,hyper=warn,hyper_util=warn,reqwest=warn,rustls=warn,tao=warn,wry=warn".into()
        }))
        .init();
    Some(guard)
}
