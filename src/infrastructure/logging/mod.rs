//! Logging module for the YOLO Dataset Cleaner application
//!
//! This module provides:
//! - Custom log formatting with bracketed output
//! - Dual logging (file + stdout)
//! - Log file management with timestamps

mod formatter;
mod setup;

// Re-export the public API
pub use setup::setup_logging;
