# YOLO Dataset Cleaner

A GUI application for efficiently managing and cleaning YOLO-format datasets. Built with Rust and egui, this tool provides an intuitive interface for reviewing, navigating, and cleaning labeled image datasets.

![YOLO Dataset Cleaner](https://img.shields.io/badge/Rust-2021-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

### 🖼️ Image Viewing & Navigation
- **Visual Dataset Browser**: View images with overlaid bounding boxes color-coded by class (CT: Blue, T: Orange)
- **Keyboard Shortcuts**: Navigate quickly with arrow keys (← Previous, → Next)
- **Auto-scaling**: Images automatically scale to fit the viewing area

### 📊 Label Information Display
- **Real-time Label Parsing**: View YOLO label data alongside images
- **Metadata Display**: See resolution, map name, and timestamp information from label comments
- **Detection Details**: View all detected objects with class, center coordinates, and dimensions
- **Detection Count**: Quick overview of how many objects are labeled in each image

### 🗂️ Dataset Management
- **Split Navigation**: Switch between train, validation, and test splits
- **Batch Deletion**: Delete images and their corresponding label files with a single action
- **Confirmation Dialog**: Prevents accidental deletions with a confirmation prompt
- **Organized Structure**: Works with standard YOLO dataset folder structure

### 🎯 YOLO Format Support
- **Standard Format**: Compatible with YOLO v5/v8 label format (class_id, x_center, y_center, width, height)
- **Metadata Comments**: Supports metadata in label files (resolution, map, timestamp)
- **Multiple Classes**: Handles multi-class datasets (CT/T for CS2 dataset)

## Installation

### Prerequisites
- **Rust**: Version 1.70 or higher ([Install Rust](https://rustup.rs/))
- **Operating System**: Windows, macOS, or Linux

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd clean-dataset

# Build the project
cargo build --release

# Run the application
cargo run --release
```

The compiled binary will be available in `target/release/clean-dataset` (or `clean-dataset.exe` on Windows).

## Usage

### Quick Start

1. **Launch the Application**
   ```bash
   cargo run --release
   ```

2. **Open Your Dataset**
   - Click the **"📁 Open Dataset Folder"** button
   - Select the root folder of your YOLO dataset (should contain `train`, `val`, and `test` subdirectories)

3. **Navigate Your Dataset**
   - Use the **Train/Val/Test** buttons to switch between splits
   - Use **◄ Previous** and **Next ►** buttons to navigate images
   - Use keyboard shortcuts: **←** (previous) and **→** (next)

4. **Review and Clean**
   - Review each image and its label information in the right panel
   - Bounding boxes are overlaid on the image with class-specific colors
   - Press **Delete** key or click **🗑 Delete Image & Label** to remove bad samples
   - Confirm deletion in the popup dialog

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

## Dependencies

This project uses the following Rust crates:

- **[eframe](https://github.com/emilk/egui)** (v0.29): Application framework
- **[egui](https://github.com/emilk/egui)** (v0.29): Immediate mode GUI library
- **[egui_extras](https://github.com/emilk/egui)** (v0.29): Additional egui utilities
- **[image](https://github.com/image-rs/image)** (v0.25): Image loading and processing
- **[rfd](https://github.com/PolyMeilex/rfd)** (v0.15): Native file dialogs
- **[kmeans_colors](https://crates.io/crates/kmeans_colors)** (v0.6): Color analysis utilities
- **[palette](https://crates.io/crates/palette)** (v0.7): Color manipulation

## Development

### Project Structure

```
clean-dataset/
├── src/
│   └── main.rs          # Main application code
├── Cargo.toml           # Project dependencies
├── Cargo.lock           # Locked dependency versions
└── README.md            # This file
```

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

The application defaults to loading the dataset from:
```
E:\CS2Vison\cs2-data-dumper\dump
```

You can modify this in `src/main.rs` at line 88, or simply use the "Open Dataset Folder" button to select a different location.

## Use Cases

### Dataset Quality Control
- Review all images to ensure labels are accurate
- Remove corrupted images or incorrect labels
- Identify and fix labeling errors before training

### Dataset Balancing
- View detection counts across splits
- Remove over-represented samples
- Balance class distribution (CT vs T)

### Dataset Cleaning
- Remove duplicate images
- Delete low-quality captures
- Clean up test data before model training

## Tips & Best Practices

1. **Backup Your Dataset**: Always keep a backup before cleaning
2. **Review Systematically**: Go through one split at a time (train → val → test)
3. **Check Edge Cases**: Pay special attention to images with 0 or many detections
4. **Use Metadata**: Filter mentally by map or resolution if looking for specific issues
5. **Keyboard Navigation**: Use arrow keys for faster navigation during review

## Troubleshooting

### Images Not Loading
- Ensure your dataset follows the correct folder structure
- Check that images are in `.png`, `.jpg`, or `.jpeg` format
- Verify that `images` and `labels` folders exist in each split

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
