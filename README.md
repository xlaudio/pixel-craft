# PixelCraft

PixelCraft is a desktop pixel-art and raster drawing application built with
Rust, [egui](https://github.com/emilk/egui), and eframe. It provides a compact
canvas workspace with layers, drawing tools, selections, image transforms, and
undo/redo.

## Features

- Brush, spray, eraser, color picker, bucket fill, line, connected lines,
  rectangle, and ellipse tools
- Rectangular, elliptical, and free-form (lasso) selections
- Move, copy, cut, delete, crop, and paste selection operations
- Layer visibility, opacity, reordering, duplication, merging, and thumbnails
- Undo and redo for document edits
- Canvas resize, flips, color inversion, and custom-angle image rotation
- PNG image workflows, layered OpenRaster (`.ora`) import/export, and PDF export
- Light, dark, and system appearance options

## Getting started

Install a stable Rust toolchain with Cargo, then run the application from the
repository root:

```sh
cargo run
```

Run the test suite with:

```sh
cargo test
```

To check formatting and compilation:

```sh
cargo fmt --check
cargo check
```

## Using the application

Choose a tool from the left toolbar and draw or select directly on the canvas.
The right-side Layers panel manages layer visibility, opacity, and ordering.
Image operations are available in the **Image** menu, while editing and
selection actions are available in **Edit**.

Some keyboard shortcuts include:

| Shortcut | Action |
| --- | --- |
| `B` | Brush |
| `A` | Spray |
| `E` | Eraser |
| `S` | Rectangular selection |
| `M` | Move |
| `P` | Connected lines |
| `Ctrl+A` | Select all |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Undo / redo |
| `Delete` | Delete the current selection |
| `Ctrl+0` | Reset zoom |
| `Ctrl+S` | Save (format follows the current filename) |

Use **File → Save as OpenRaster...** to export an editable `.ora` document
with its layer order, visibility, and opacity preserved. **Save** and `Ctrl+S`
keep using OpenRaster when the current filename ends in `.ora`.

## Project structure

The document and editing operations are in `src/app/` and `src/model.rs`.
Commands provide a shared entry point for application actions. The native
egui/eframe interface is in `src/ui/`. See [ARCHITECTURE.md](./ARCHITECTURE.md)
for more detail.

## Contributing

Bug reports and contributions are welcome. Please read
[CONTRIBUTING.md](./CONTRIBUTING.md) before submitting changes.

## License

No license for PixelCraft is currently declared in this repository. See
[LICENSING.md](./LICENSING.md) for details.
