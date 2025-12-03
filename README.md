# YOLO Dataset Cleaner

A GUI application for efficiently managing and cleaning YOLO-format datasets. Built with Rust and egui, this tool provides an intuitive interface for reviewing, navigating, and cleaning labeled image datasets.

![YOLO Dataset Cleaner](https://img.shields.io/badge/Rust-2021-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

### 🖼️ Image Viewing & Navigation
- **Visual Dataset Browser**: View images with overlaid bounding boxes color-coded by class (T: Orange, CT: Blue)
- **Keyboard Shortcuts**: Navigate quickly with arrow keys (← Previous, → Next)
- **Auto-scaling**: Images automatically scale to fit the viewing area
- **Slider-like Navigation**: Swiper-style previous/next buttons positioned on left and right sides of images
- **Loading States**: Visual feedback with loading indicators during image loading

### 📊 Label Information Display
- **Real-time Label Parsing**: View YOLO label data alongside images
- **Metadata Display**: See resolution, map name, and timestamp information from label comments
- **Detection Details**: View all detected objects with class, center coordinates, and dimensions
- **Detection Count**: Quick overview of how many objects are labeled in each image
- **Dominant Color Indicator**: Shows the dominant color of the current image for quality assessment

### 🗂️ Dataset Management
- **Split Navigation**: Switch between train, validation, and test splits
- **Individual Deletion**: Delete images and their corresponding label files with a single action
- **Undo/Redo System**: Undo deletions within 3 seconds with automatic finalization
- **Confirmation Dialog**: Prevents accidental deletions with a confirmation prompt
- **Organized Structure**: Works with standard YOLO dataset folder structure
- **Toast Notifications**: Visual feedback for successful actions like deletions and undos

### 🧹 Batch Operations
- **Black Image Removal**: Automatically detect and remove images with black or near-black dominant colors
- **K-Means Color Analysis**: Uses advanced k-means clustering in LAB color space for accurate color detection
- **Batch Processing**: Scan entire splits and remove multiple images at once
- **Progress Tracking**: Real-time progress display during batch operations with cancel support
- **Statistics Report**: View detailed results including total scanned, deleted, and retention rate

### 🎯 YOLO Format Support
- **Standard Format**: Compatible with YOLO v5/v8 label format (class_id, x_center, y_center, width, height)
- **Metadata Comments**: Supports metadata in label files (resolution, map, timestamp)
- **Multiple Classes**: Handles multi-class datasets (T/CT for CS2 dataset)

### 📝 Logging & Debugging
- **Structured Logging**: Comprehensive logging system using `tracing` and `tracing-subscriber`
- **Custom Log Format**: Bracketed formatter with timestamps, log levels, function names, and source locations
- **File Logging**: All logs saved to timestamped files in the `logs/` directory
- **Selective Filtering**: Reduced noise from third-party libraries (egui, eframe, winit)

### 💾 Persistent Settings
- **Auto-Save Preferences**: Automatically remembers your last opened dataset, active split, and image position
- **Window Size Memory**: Restores window dimensions between sessions
- **Portable Settings**: Settings file stored next to the executable for easy backup and portability
- **Seamless Experience**: Pick up right where you left off when reopening the application

## Installation

### Prerequisites
- **Rust**: Version 1.70 or higher ([Install Rust](https://rustup.rs/))
- **Operating System**: Windows, macOS, or Linux

### Build from Source

```bash
# Clone the repository
git clone https://github.com/ALPhaHoai/clean-cs2vision-dataset.git
cd clean-cs2vision-dataset

# Build the project
cargo build --release

# Run the application
cargo run --release
```

The compiled binary will be available in `target/release/clean-cs2vision-dataset` (or `clean-cs2vision-dataset.exe` on Windows).

## Usage

### Quick Start

1. **Launch the Application**
   ```bash
   cargo run --release
   ```

2. **Try the Sample Dataset** (Optional but Recommended)
   - The application includes a `sample-dataset/` folder with Ghibli-style images
   - Click **"📁 Open Dataset Folder"** and select the `sample-dataset` directory
   - This lets you explore all features without risking your real data

3. **Open Your Dataset**
   - Click the **"📁 Open Dataset Folder"** button
   - Select the root folder of your YOLO dataset (should contain `train`, `val`, and `test` subdirectories)

4. **Navigate Your Dataset**
   - Use the **Train/Val/Test** buttons to switch between splits
   - Use **◄ Previous** and **Next ►** buttons to navigate images (slider-style positioned on image sides)
   - Use keyboard shortcuts: **←** (previous) and **→** (next)

5. **Review and Clean**
   - Review each image and its label information in the right panel
   - Bounding boxes are overlaid on the image with class-specific colors
   - Press **Delete** key or click **🗑 Delete Image & Label** to remove bad samples
   - Confirm deletion in the popup dialog
   - Toast notification appears on successful deletion
   - Press **Ctrl+Z** within 3 seconds to undo if needed

6. **Batch Remove Black Images**
   - Click **🧹 Remove Black Images** button to detect and remove images with black/near-black content
   - Review the confirmation dialog showing split and total image count
   - Confirm to start the batch processing
   - Monitor progress in real-time as images are scanned
   - View final statistics including total scanned, deleted, and retention rate

### Dataset Structure

Your dataset should follow this structure:

```
dataset/
├── train/
│   ├── images/
│   │   ├── image001.png
│   │   ├── image002.png
│   │   └── ...
│   └── labels/
│       ├── image001.txt
│       ├── image002.txt
│       └── ...
├── val/
│   ├── images/
│   └── labels/
└── test/
    ├── images/
    └── labels/
```

### Sample Dataset

The project includes a `sample-dataset/` directory for testing and learning:

**Contents:**
- **Train Split**: 10 Ghibli-style anime images with YOLO labels
- **Val Split**: Additional validation images
- **Test Split**: Additional test images

**Purpose:**
- Explore the application's features without needing your own dataset
- Test batch operations safely
- Learn keyboard shortcuts and navigation
- Understand the YOLO label format and how it's displayed
- Practice the undo/redo functionality

**Images:** The sample images are Ghibli-style artwork generated for demonstration purposes, labeled with example bounding boxes to show how the tool displays and manages YOLO datasets.

### Label Format

Label files (`.txt`) should follow the YOLO format:

```
# Resolution: 2560x1440, Map: de_dust2, Time: 1764637338
0 0.5234 0.4512 0.1234 0.2345
1 0.7823 0.6234 0.0987 0.1876
```

- **Comment line** (optional): Metadata about the image
- **Detection lines**: `class_id x_center y_center width height` (normalized 0-1)

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **←** | Previous image |
| **→** | Next image |
| **Delete** | Open delete confirmation dialog |
| **Ctrl+Z** | Undo last deletion (within 3 seconds) |

## Dependencies

This project uses the following Rust crates:

- **[eframe](https://github.com/emilk/egui)** (v0.29): Application framework
- **[egui](https://github.com/emilk/egui)** (v0.29): Immediate mode GUI library
- **[egui_extras](https://github.com/emilk/egui)** (v0.29): Additional egui utilities
- **[egui-phosphor](https://crates.io/crates/egui-phosphor)** (v0.7): Phosphor icon library for egui
- **[image](https://github.com/image-rs/image)** (v0.25): Image loading and processing
- **[rfd](https://github.com/PolyMeilex/rfd)** (v0.15): Native file dialogs
- **[kmeans_colors](https://crates.io/crates/kmeans_colors)** (v0.6): Color analysis utilities
- **[palette](https://crates.io/crates/palette)** (v0.7): Color manipulation
- **[tracing](https://crates.io/crates/tracing)** (v0.1): Structured logging
- **[tracing-subscriber](https://crates.io/crates/tracing-subscriber)** (v0.3): Logging implementation with env-filter support
- **[chrono](https://crates.io/crates/chrono)** (v0.4): Date and time handling for log timestamps
- **[serde](https://crates.io/crates/serde)** (v1.0): Serialization framework for settings persistence
- **[serde_json](https://crates.io/crates/serde_json)** (v1.0): JSON serialization for settings files

## Development

### Project Structure

```
clean-cs2vision-dataset/
├── src/
│   ├── main.rs              # Application entry point and core logic
│   ├── config.rs            # Centralized configuration management
│   ├── dataset.rs           # Dataset loading and management
│   ├── label_parser.rs      # YOLO label file parsing
│   ├── image_analysis.rs    # Image color analysis and black detection
│   ├── log_formatter.rs     # Custom bracketed log formatter
│   ├── settings.rs          # Persistent user settings management
│   └── ui/                  # User interface modules
│       ├── mod.rs           # UI module exports
│       ├── panels.rs        # UI panels (top, bottom, label, central)
│       ├── keyboard.rs      # Keyboard shortcut handling
│       ├── batch_dialogs.rs # Batch operation dialogs and progress
│       ├── image_renderer.rs # Image rendering with bounding boxes
│       └── toast.rs         # Toast notification system
├── sample-dataset/          # Sample dataset for testing
│   ├── train/               # Training split with Ghibli-style images
│   ├── val/                 # Validation split
│   └── test/                # Test split
├── logs/                    # Timestamped log files
├── Cargo.toml               # Project dependencies
├── Cargo.lock               # Locked dependency versions
├── build_release.bat        # Windows build script
└── README.md                # This file
```

### Architecture

The application follows a modular architecture:

- **`config.rs`**: Centralizes all configuration values (colors, paths, window sizes) in a single location
- **`dataset.rs`**: Handles dataset loading, split management, and file operations
- **`label_parser.rs`**: Parses YOLO label files and extracts metadata
- **`image_analysis.rs`**: Provides image color analysis using k-means clustering in LAB color space. Includes functions to calculate dominant colors and detect black/near-black images
- **`log_formatter.rs`**: Custom log formatter that wraps log fields in brackets for improved readability (timestamp, level, function, location)
- **`settings.rs`**: Manages persistent user preferences (dataset path, window size, split, image index). Settings are saved as JSON next to the executable for portability
- **`ui/`**: Contains all UI-related code, separated by functionality:
  - `panels.rs`: Renders all UI panels (navigation, labels, image display) using Phosphor icons
  - `keyboard.rs`: Handles keyboard input and shortcuts (navigation, delete, undo)
  - `batch_dialogs.rs`: Manages batch operation dialogs (confirmation, progress, results)
  - `image_renderer.rs`: Renders images with overlaid bounding boxes
  - `toast.rs`: Toast notification system for user feedback

### Building for Development

```bash
# Run in debug mode (faster compilation)
cargo run

# Run with optimizations (faster runtime)
cargo run --release

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy
```

### Configuration

The application uses a centralized configuration system in `src/config.rs`. All settings are defined in the `AppConfig` struct:

```rust
pub struct AppConfig {
    pub default_dataset_path: PathBuf,    // Default dataset location
    pub window_width: f32,                 // Initial window width
    pub window_height: f32,                // Initial window height
    pub class_names: Vec<&'static str>,   // Class names (T, CT)
    pub class_colors: Vec<(Color32, Color32)>, // Border and fill colors
    pub side_panel_width: f32,             // Right panel width
}
```

#### Default Values

- **Dataset Path**: `E:\CS2Vison\cs2-data-dumper\dump`
- **Window Size**: 1200x800 pixels
- **Side Panel Width**: 300 pixels
- **Class 0 (T)**: Orange border (RGB: 255, 140, 0)
- **Class 1 (CT)**: Blue border (RGB: 100, 149, 237)

To customize these values, edit `src/config.rs` in the `Default` implementation. You can also select a different dataset location at runtime using the "📁 Open Dataset Folder" button.

**Note**: User preferences (last dataset opened, window size, active split, current image index) are automatically saved in a `settings.json` file next to the executable and restored on next launch.

### Logging Configuration

The application uses structured logging with custom formatting:

- **Log Format**: `[TIMESTAMP] [LEVEL] [FUNCTION_NAME] [TARGET: FILE:LINE]: MESSAGE`
- **Log Directory**: `logs/` (created automatically)
- **Log Files**: Timestamped format `app_YYYYMMDD_HHMMSS.log`
- **Log Levels**: Configured via `RUST_LOG` environment variable (defaults to `info` for the app, filters out trace/debug from third-party libs)
- **Custom Formatter**: Implemented in `src/log_formatter.rs` for enhanced readability

Logs include detailed information about image operations, deletions, undo actions, and error handling.

## Use Cases

### Dataset Quality Control
- Review all images to ensure labels are accurate
- Remove corrupted images or incorrect labels
- Identify and fix labeling errors before training

### Dataset Balancing
- View detection counts across splits
- Remove over-represented samples
- Balance class distribution (T vs CT)

### Dataset Cleaning
- Remove duplicate images
- Delete low-quality captures
- Clean up test data before model training

### Batch Black Image Removal
- Automatically detect and remove loading screens or black frames
- Clean datasets with many corrupted or failed captures
- Remove images from game crashes or screen transitions
- Improve dataset quality by filtering out near-black images (RGB < 10)

## Tips & Best Practices

1. **Backup Your Dataset**: Always keep a backup before cleaning, especially before batch operations
2. **Review Systematically**: Go through one split at a time (train → val → test)
3. **Check Edge Cases**: Pay special attention to images with 0 or many detections
4. **Use Metadata**: Filter mentally by map or resolution if looking for specific issues
5. **Keyboard Navigation**: Use arrow keys for faster navigation during review
6. **Undo Deletions**: Accidentally deleted something? Press **Ctrl+Z** within 3 seconds to undo
7. **Batch Black Image Removal**: Run this on each split separately after initial data collection to remove failed captures
8. **Monitor Dominant Color**: The dominant color indicator in the label panel helps identify problematic images before batch processing
9. **Check Logs**: Review log files in the `logs/` directory to troubleshoot issues or audit operations
10. **Test with Sample Dataset**: Use the included `sample-dataset/` to familiarize yourself with the tool before working on your actual data

## Troubleshooting

### Images Not Loading
- Ensure your dataset follows the correct folder structure
- Check that images are in `.png`, `.jpg`, or `.jpeg` format
- Verify that `images` and `labels` folders exist in each split
- Check the `logs/` directory for detailed error messages about image decoding failures
- If an image fails to load, the app displays an error message instead of showing a loading spinner indefinitely

### Labels Not Displaying
- Confirm label files are in the `labels` folder (not `images`)
- Check that label filenames match image filenames (except extension)
- Ensure label files use `.txt` extension

### Bounding Boxes Incorrect
- Verify YOLO format coordinates are normalized (0-1 range)
- Check that coordinates use center format (not corner format)
- Ensure width and height values are positive

## License

This project is available under the MIT License. See the LICENSE file for more details.

## Acknowledgments

Built for CS2 YOLO dataset management, supporting efficient review and cleaning of player detection datasets.

---

**Made with ❤️ using Rust and egui**
