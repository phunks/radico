
use tracing_appender::{self, rolling::daily};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter, Registry};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[allow(dead_code)]
pub struct Logger {
    _guard: WorkerGuard,
}

#[allow(dead_code)]
impl Logger {
    pub fn build(verbosity: u8) -> Self {
        let file_appender = daily("/tmp", "debug_radico.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
            match verbosity {
                0 => "warn".to_string(),
                1 => "info".to_string(),
                2 => "debug".to_string(),
                _ => "trace".to_string(),
            }
        });

        let file_layer = fmt::layer()
            .json()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(verbosity > 1)
            .with_level(verbosity > 2)
            .with_thread_ids(verbosity > 3)
            .with_thread_names(verbosity > 4);

        Registry::default()
            .with(EnvFilter::new(filter))
            .with(file_layer)
            .init();

        Self { _guard: guard }
    }
}