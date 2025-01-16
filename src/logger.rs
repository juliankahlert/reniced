use atty::Stream;
use tracing::Level;
use tracing::{self};
use tracing_journald::layer as journald_layer;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Logs a message at the `debug` level.
///
/// This macro is used to log messages that are useful for debugging and development. Messages logged at this level
/// provide detailed information useful for diagnosing issues in the code, but they may be too verbose for production.
///
/// # Example
/// ```rust
/// debug!("This is a debug message with value: {}", value);
/// ```
///
/// # Note
/// This will only be captured if the logging level includes "debug" or lower (e.g., "trace").
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        tracing::debug!($($arg)*);
    };
}

/// Logs a message at the `info` level.
///
/// This macro is used for general information that is useful for tracking the progress of the application.
/// Messages logged at the `info` level are generally the most common type of logs, and they represent normal application
/// operations or significant milestones.
///
/// # Example
/// ```rust
/// info!("The application has started successfully.");
/// ```
///
/// # Note
/// This will be captured if the logging level includes "info", "warn", or "error".
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*);
    };
}

/// Logs a message at the `warn` level.
///
/// This macro is used to log messages that indicate potential issues or warnings in the system. While these messages do not
/// indicate an error, they represent situations that might require attention, such as unexpected behavior or conditions that
/// may lead to future problems.
///
/// # Example
/// ```rust
/// warn!("The system is using a fallback configuration.");
/// ```
///
/// # Note
/// This will be captured if the logging level includes "warn" or "error".
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
    };
}

/// Logs a message at the `error` level.
///
/// This macro is used for logging errors or exceptional conditions that may affect the functionality of the application.
/// Messages logged at the `error` level are typically used to capture events that require immediate attention or intervention.
///
/// # Example
/// ```rust
/// error!("Failed to load configuration file: {}", file_path);
/// ```
///
/// # Note
/// This will be captured if the logging level includes "error".
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*);
    };
}

/// Logs a message at the `trace` level.
///
/// This macro is used for very fine-grained, detailed logs that are typically used during debugging or tracing execution flow.
/// These messages can provide a very detailed level of insight into the application's behavior, but they are usually too verbose
/// for regular use in production environments.
///
/// # Example
/// ```rust
/// trace!("Tracing function entry: {}", function_name);
/// ```
///
/// # Note
/// This will be captured if the logging level is set to "trace".
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        tracing::trace!($($arg)*);
    };
}

/// Initializes the logger with support for journald and stdout logging.
///
/// # Arguments
///
/// * `log_level` - A string that represents the log level (e.g., "info", "debug", "error").
/// * `foreground` - A boolean indicating whether the application is running in the foreground.
///
/// # Description
///
/// This function initializes the logging system with journald and/or stdout logging based on
/// whether the application is running in the foreground or background. If journald is available,
/// logs will be sent to the system journal. If running in the foreground, logs will also
/// be printed to stdout.
pub fn init_logger(log_level: Option<&str>) {
    let foreground = atty::is(Stream::Stdout);
    let level_str = log_level.unwrap_or("info");

    let level = match level_str.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => {
            eprintln!("Invalid log level provided. Defaulting to 'info'.");
            Level::INFO
        }
    };

    let journald_available = journald_layer().is_ok();

    match (journald_available, foreground) {
        (true, true) => init_with_journald_and_foreground(level),
        (true, false) => init_with_journald(level),
        (false, true) => init_with_foreground(level),
        (false, false) => init_with_foreground(level),
    }

    debug!(
        "Logging initialized at {} level. Journald logging: {}, Stdout logging: {}",
        level_str,
        if journald_available {
            "enabled"
        } else {
            "disabled"
        },
        if foreground { "enabled" } else { "disabled" }
    );
}

/// Initializes the logger with both journald and stdout logging.
///
/// This function configures the logger to send logs to the system journal using journald and
/// also prints logs to the standard output (stdout) when the application is running in the foreground.
/// It takes the provided log level and applies it to both logging outputs.
///
/// # Arguments
///
/// * `level` - The log level (e.g., `Level::INFO`, `Level::ERROR`) to be used for logging.
///
/// # Description
///
/// This function creates two layers:
/// - A `journald_layer` that sends logs to the system journal.
/// - A `fmt_layer` that prints logs to stdout.
/// The two layers are then added to the `tracing_subscriber::registry` along with the level filter.
fn init_with_journald_and_foreground(level: Level) {
    let fmt_layer = fmt::layer().with_target(false);

    // Safe unwrap because we checked availability
    let journald_layer = journald_layer().unwrap();

    tracing_subscriber::registry()
        .with(journald_layer)
        .with(fmt_layer)
        .with(tracing_subscriber::filter::LevelFilter::from(level))
        .init();
}

/// Initializes the logger with journald logging only.
///
/// This function configures the logger to send logs only to the system journal using journald,
/// and does not print logs to stdout, regardless of whether the application is running in the foreground.
///
/// # Arguments
///
/// * `level` - The log level (e.g., `Level::INFO`, `Level::ERROR`) to be used for logging.
///
/// # Description
///
/// This function creates a `journald_layer` that sends logs to the system journal.
/// The layer is then added to the `tracing_subscriber::registry` along with the level filter.
fn init_with_journald(level: Level) {
    // Safe unwrap because we checked availability
    let journald_layer = journald_layer().unwrap();

    tracing_subscriber::registry()
        .with(journald_layer)
        .with(tracing_subscriber::filter::LevelFilter::from(level))
        .init();
}

/// Initializes the logger with stdout logging only (foreground mode).
///
/// This function configures the logger to print logs to stdout, which is typically used when
/// the application is running in the foreground (e.g., when executed in a terminal).
///
/// # Arguments
///
/// * `level` - The log level (e.g., `Level::INFO`, `Level::ERROR`) to be used for logging.
///
/// # Description
///
/// This function creates a `fmt_layer` that prints logs to the standard output (stdout).
/// The layer is then added to the `tracing_subscriber::registry` along with the level filter.
fn init_with_foreground(level: Level) {
    let fmt_layer = fmt::layer().with_target(false);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::filter::LevelFilter::from(level))
        .init();
}

/// Initializes the logger with no output (neither journald nor stdout).
///
/// This function configures the logger with no output, meaning no logs will be sent to journald
/// and no logs will be printed to stdout.
///
/// # Arguments
///
/// * `level` - The log level (e.g., `Level::INFO`, `Level::ERROR`) to be used for logging.
///
/// # Description
///
/// This function adds only the level filter to the `tracing_subscriber::registry`, without any
/// output layers, effectively silencing the logger. This is typically used when logging is disabled
/// for certain environments.
fn _init_without_logging(level: Level) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from(level))
        .init();
}
