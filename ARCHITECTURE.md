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
execute commands with `CommandEngine::execute`. The engine returns notices
instead of storing presentation messages in the document. They do not need an
egui window, an HTTP service, or an MCP server.
`PaintApp::execute_command` is a convenience entry point for the same
dispatcher and returns the same notices. Document fields are not publicly
mutable; adapters inspect the document through read-only accessors and make
changes through commands, so they cannot bypass revision tracking.

The native adapter is `ui::EframeApp`: it owns a `PaintApp` document and a
separate `ui::UiState`. Viewport, pointer interaction, dialogs, palette,
notifications, and the texture cache never enter the document model.
`PaintApp::document_revision()` advances on document edits; the adapter compares
that revision with the texture cache to decide when to recomposite, rather
than setting a GUI dirty flag from document operations.

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
| `src/app/document.rs` | Canvas sizing and compositing |
| `src/app/document/selection.rs` | Selection editing, selection moves, and clipboard operations |
| `src/app/document/layers.rs` | Layer merging and pixel-level transformations |
| `src/app/history.rs` | Undo/redo snapshots |
| `src/app/tools.rs` | Raster drawing and paint algorithms |
| `src/app/file_io.rs` | Image/PDF loading and saving; paths are supplied by the caller |
| `src/app/mod.rs` | Headless document state construction |
| `src/model.rs` | Layers, tools, history snapshots, and `PaintApp` document state |
| `src/ui/eframe_app.rs` | eframe adapter, UI state, rendering cache, and keyboard translation |
| `src/ui/` | egui views; widgets gather input and dispatch commands |

File pickers and dialogs remain GUI adapter concerns. They provide paths or
command values to the engine instead of being opened by document commands.
The `Command` payloads use plain coordinates and RGBA channels so transport
adapters can map their own request formats without exposing egui widgets.
