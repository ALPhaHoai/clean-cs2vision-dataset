# YOLO Dataset Cleaner

A GUI application for efficiently managing and cleaning YOLO-format datasets. Built with Rust and egui, this tool provides an intuitive interface for reviewing, navigating, and cleaning labeled image datasets.

![YOLO Dataset Cleaner](https://img.shields.io/badge/Rust-2021-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

### 🖼️ Image Viewing & Navigation
- **Visual Dataset Browser**: View images with overlaid bounding boxes color-coded by class (T: Orange, CT: Blue)
- **Keyboard Shortcuts**: Navigate quickly with arrow keys (← Previous, → Next)
- **Auto-scaling**: Images automatically scale to fit the viewing area
- **Zoom Controls**: Zoom in/out on images using Ctrl + mouse wheel (50%-300%)
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
- **Unlimited Undo/Redo Stack**: Undo and redo multiple deletions with full history (no timeout)
- **Confirmation Dialog**: Prevents accidental deletions with a confirmation prompt
- **Organized Structure**: Works with standard YOLO dataset folder structure
- **Toast Notifications**: Visual feedback showing undo/redo availability and action counts

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

### 🔍 Image Filtering
- **Team Filters**: Filter images by team presence (All, T Only, CT Only, Both, T Exclusive, CT Exclusive)
- **Player Count Filters**: Filter by player count (Any, Single, Multiple 2+, Background/No Players)
- **Real-time Preview**: See filtered image count before applying
- **Filter Dialog**: Dedicated UI for configuring filters (Ctrl+F)
- **Filtered Navigation**: Navigate through filtered results seamlessly

### 📊 Dataset Balance Analyzer
- **Distribution Analysis**: Analyze dataset by player types (CT Only, T Only, Multiple Players, Background, Hard Cases)
- **Progress Tracking**: Real-time progress display with cancel support during analysis
- **Target Ratios**: Compare current distribution against target ratios (85% players, 10% background, 5% hard cases)
- **Smart Recommendations**: Get actionable suggestions for balancing your dataset
- **Detailed Breakdown**: View percentages and counts for each category

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
   - Toast notification shows undo/redo availability and counts
   - Press **Ctrl+Z** to undo or **Ctrl+Y** to redo (unlimited history)

6. **Filter Images** (Optional)
   - Press **Ctrl+F** or click the filter button to open the filter dialog
   - Select team filter: All Teams, T Only, CT Only, Both T & CT, T Exclusive, or CT Exclusive
   - Select player count filter: Any, Single, Multiple (2+), or Background (No Players)
   - Click **Apply Filters** to see only images matching your criteria
   - Navigate through filtered results using arrow keys or buttons
   - Click **Clear All** to remove filters and see all images again

7. **Analyze Dataset Balance**
   - Click **📊 Analyze Balance** button to analyze your dataset distribution
   - Monitor progress as the tool scans all images and categorizes them
   - Review the analysis results showing:
     - Player images breakdown (CT Only, T Only, Multiple Players)
     - Background images count
     - Hard cases (both teams, no player, or ambiguous)
   - Compare current distribution against target ratios
   - Follow recommendations to improve dataset balance

8. **Batch Remove Black Images**
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

#### Navigation
| Key | Action |
|-----|--------|
| **←** | Previous image |
| **→** | Next image |
| **Home** | Jump to first image |
| **End** | Jump to last image |
| **Page Up** | Jump backward 10 images |
| **Page Down** | Jump forward 10 images |
| **1** | Switch to Train split |
| **2** | Switch to Val split |
| **3** | Switch to Test split |

#### Zoom Controls
| Key | Action |
|-----|--------|
| **Ctrl + Mouse Wheel** | Zoom in/out |
| **Ctrl + 0** | Reset zoom to 100% |
| **Ctrl + =** (plus) | Zoom in by 10% |
| **Ctrl + -** (minus) | Zoom out by 10% |

#### Actions
| Key | Action |
|-----|--------|
| **Delete** | Delete current image & label |
| **Ctrl+Z** | Undo last deletion |
| **Ctrl+Y** | Redo last undone deletion |
| **Ctrl+Shift+Z** | Redo (alternative shortcut) |
| **Space** | Toggle fullscreen mode |
| **Escape** | Close dialogs / Exit fullscreen |
| **Ctrl+O** | Open dataset folder |
| **Ctrl+F** | Open filter dialog |

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
- **[directories](https://crates.io/crates/directories)** (v5.0): Standard directory paths across platforms

## Development

### Project Structure

```
clean-cs2vision-dataset/
├── src/
│   ├── main.rs              # Application entry point (slim)
│   ├── app.rs               # Main application logic and DatasetCleanerApp
│   ├── core/                # Core business logic
│   │   ├── mod.rs
│   │   ├── filter.rs        # Image filtering by team and player count
│   │   ├── analysis/        # Dataset analysis
│   │   │   ├── mod.rs
│   │   │   └── balance_analyzer.rs  # Balance analysis and recommendations
│   │   ├── dataset/         # Dataset management
│   │   │   ├── mod.rs
│   │   │   ├── dataset.rs   # Dataset loading and split management
│   │   │   └── label.rs     # YOLO label file parsing
│   │   ├── image/           # Image processing
│   │   │   ├── mod.rs
│   │   │   └── analysis.rs  # Image color analysis and black detection
│   │   └── operations/      # File operations
│   │       ├── mod.rs
│   │       └── file_ops.rs  # Delete, move, and file path utilities
│   ├── state/               # State management
│   │   ├── mod.rs
│   │   ├── app_state.rs     # ImageState, UIState, BatchState, FilterState, etc.
│   │   ├── settings.rs      # Persistent user settings
│   │   └── undo_manager.rs  # Undo/redo stack management
│   ├── ui/                  # User interface components
│   │   ├── mod.rs
│   │   ├── panels.rs        # UI panels (top, bottom, label, central)
│   │   ├── keyboard.rs      # Keyboard shortcut handling
│   │   ├── batch_dialogs.rs # Batch operation dialogs and progress
│   │   ├── balance_dialog.rs # Balance analysis dialog
│   │   ├── filter_dialog.rs # Filter configuration dialog
│   │   ├── image_renderer.rs # Image rendering with bounding boxes
│   │   └── toast.rs         # Toast notification system
│   ├── infrastructure/      # Infrastructure concerns
│   │   ├── mod.rs
│   │   └── logging/         # Logging configuration and formatters
│   │       ├── mod.rs
│   │       ├── formatter.rs # Custom bracketed log formatter
│   │       └── setup.rs     # Logger initialization
│   └── config/              # Configuration management
│       ├── mod.rs
│       └── app_config.rs    # Centralized configuration
├── sample-dataset/          # Sample dataset for testing
│   ├── train/               # Training split with Ghibli-style images
│   ├── val/                 # Validation split
│   └── test/                # Test split
├── logs/                    # Timestamped log files
├── settings.json            # User settings (auto-generated)
├── Cargo.toml               # Project dependencies
├── Cargo.lock               # Locked dependency versions
├── build_release.bat        # Windows build script
└── README.md                # This file
```

### Architecture

The application follows a clean, modular architecture with separation of concerns:

#### Core Modules (`src/core/`)
Business logic and domain operations:
- **`filter.rs`**: Image filtering logic with team and player count criteria
- **`analysis/balance_analyzer.rs`**: Dataset balance analysis, categorization, and recommendations
- **`dataset/dataset.rs`**: Dataset loading, split management, and image listing
- **`dataset/label.rs`**: YOLO label file parsing and metadata extraction
- **`image/analysis.rs`**: Image color analysis using k-means clustering in LAB color space
- **`operations/file_ops.rs`**: File operations (delete, move, path utilities)

#### State Management (`src/state/`)
Centralized state structs for application data:
- **`app_state.rs`**: Core state structs (ImageState, UIState, BatchState, BalanceAnalysisState, FilterState)
- **`settings.rs`**: Persistent user preferences (dataset path, window size, split, image index)
- **`undo_manager.rs`**: Stack-based undo/redo management with unlimited history

#### User Interface (`src/ui/`)
UI components and rendering:
- **`panels.rs`**: Main UI panels (navigation, labels, image display) using Phosphor icons
- **`keyboard.rs`**: Keyboard input handling and shortcuts
- **`batch_dialogs.rs`**: Batch operation dialogs (confirmation, progress, results)
- **`balance_dialog.rs`**: Balance analysis results dialog with recommendations
- **`filter_dialog.rs`**: Filter configuration dialog with team and player count options
- **`image_renderer.rs`**: Image rendering with overlaid bounding boxes
- **`toast.rs`**: Toast notification system for undo/redo feedback

#### Infrastructure (`src/infrastructure/`)
- **`logging/`**: Structured logging with custom bracketed formatter and file output

#### Configuration (`src/config/`)
- **`app_config.rs`**: Centralized configuration (colors, paths, window sizes, target ratios)

#### Application Entry (`src/`)
- **`main.rs`**: Slim entry point handling app initialization
- **`app.rs`**: Main `DatasetCleanerApp` struct with eframe::App implementation

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
6. **Undo/Redo Deletions**: Accidentally deleted something? Press **Ctrl+Z** to undo (unlimited history). Press **Ctrl+Y** to redo if needed
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
