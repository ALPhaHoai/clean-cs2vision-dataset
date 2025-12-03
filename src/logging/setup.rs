use std::fs;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use super::formatter::BracketedFormatter;

pub fn setup_logging() -> PathBuf {
    // Create logs directory
    let log_dir = std::env::current_dir().unwrap().join("logs");
    fs::create_dir_all(&log_dir).expect("Failed to create logs directory");

    // Create log file with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("dataset_cleaner_{}.log", timestamp);
    let log_path = log_dir.join(&log_filename);

    // Create file appender
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .expect("Failed to create log file");

    let file_layer = fmt::layer()
        .event_format(BracketedFormatter)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false); // Disable ANSI colors in file

    let stdout_layer = fmt::layer()
        .event_format(BracketedFormatter)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            // Keep our app at trace level, but suppress verbose third-party logs
            EnvFilter::new("trace")
                .add_directive("winit=warn".parse().unwrap())
                .add_directive("log=warn".parse().unwrap())
                .add_directive("egui=warn".parse().unwrap())
                .add_directive("eframe=warn".parse().unwrap())
        }))
        .with(file_layer)
        .with(stdout_layer)
        .init();

    info!("Starting YOLO Dataset Cleaner application");
    info!("Log file created at: {:?}", log_path);

    log_path
}
