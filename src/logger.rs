use thiserror::Error;
use tracing_subscriber::prelude::*;

#[derive(Debug, Error)]
pub enum LoggerError {
    #[error("Failed to initialize logger: {0}")]
    InitError(String),
}

pub fn init_logger(config: &crate::config::Config) -> Result<(), LoggerError> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(config.log_level()));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if config.json_output() {
        subscriber
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .init();
    } else {
        subscriber
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_file(false)
                    .with_line_number(false)
                    .pretty(),
            )
            .init();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_initializes_works() {
        let config = crate::config::Config::default();
        let result = init_logger(&config);
        assert!(result.is_ok());
    }
}
