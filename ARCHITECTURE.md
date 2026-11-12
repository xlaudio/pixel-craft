# Application architecture

All application-level actions enter through `app::CommandEngine`. The GUI
translates egui events into `app::Command` values; later scripting, HTTP, or
MCP adapters can translate their own requests into those same values and call
the same engine against the same application state. An MCP server is not part
of this project.

```text
egui UI / keyboard ─────┐
future scripting ───────┤
future HTTP adapter ────┼──> app::Command ──> app::CommandEngine
future MCP adapter ─────┘                           │
                                                    v
                                   document / history / tools / file I/O
                                                    │
                                                    v
                                        shared PaintApp state
```

Non-GUI adapters can create the state with `PaintApp::new_headless()` and
execute commands with `CommandEngine::execute`. They do not need an egui
window, an HTTP service, or an MCP server. The GUI uses
`PaintApp::execute_command` as its convenience entry point.

```rust
use pixel_craft::{
    app::{Command, CommandEngine},
    model::{PaintApp, Tool},
};

let mut state = PaintApp::new_headless();
let mut engine = CommandEngine::default();
engine.execute(&mut state, Command::SetTool(Tool::Brush));
```

## Rust modules

| Module | Responsibility |
| --- | --- |
| `src/lib.rs` | Public library boundary shared with the native binary and future adapters |
| `src/main.rs` | Native eframe window entry point |
| `src/app/commands.rs` | Shared command types and the command dispatcher |
| `src/app/document.rs` | Canvas, selections, clipboard, transforms, and compositing |
| `src/app/history.rs` | Undo/redo snapshots |
| `src/app/tools.rs` | Raster drawing and paint algorithms |
| `src/app/file_io.rs` | Image/PDF loading and saving; paths are supplied by the caller |
| `src/app/mod.rs` | Application state construction, shared command entry point, and egui integration helpers |
| `src/model.rs` | Layers, tools, history snapshots, and application state |
| `src/ui/eframe_app.rs` | Frame orchestration and keyboard-to-command translation |
| `src/ui/` | egui views; widgets gather input and dispatch commands |

File pickers and dialogs remain GUI adapter concerns. They provide paths or
command values to the engine instead of being opened by document commands.
The `Command` payloads use plain coordinates and RGBA channels so transport
adapters can map their own request formats without exposing egui widgets.
