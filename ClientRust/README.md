# Mir2 Rust Client (WIP)

This directory contains an experimental Rust rewrite of the Crystal Mir2 client.
The goal is to provide a modern, safe, and portable client while keeping parity
with the original C# implementation.

## Project layout

- `Cargo.toml` – workspace manifest and dependency list.
- `src/main.rs` – entry point that bootstraps the runtime.
- `src/settings.rs` – configuration loader mirroring `Settings.cs`.
- `src/runtime.rs` – initial orchestration for async runtime + subsystems.
- `src/net.rs` – async TCP networking placeholder.
- `src/audio.rs` – basic audio engine stub using `rodio`.
- `src/ui.rs` – temporary text-mode event loop that will be replaced with the
  actual rendering/UI layer.
- `src/error.rs` – shared error type.

## Getting started

```powershell
# From the repo root
cd ClientRust
cargo run
```

The current implementation starts the async runtime, attempts to connect to the
configured game server, and prints network events. Rendering, input handling,
resource loading, and full protocol support will be added incrementally.

## Configuration

- The loader now understands the legacy `Mir2Config.ini` / `Mir2Test.ini` files
   from the C# client. Pass `-tc` (or set `MIR2_CLIENT_USE_TEST_CONFIG=true`) to
   mirror the old test-config toggle.
- Modern formats (`config/client.json`, `.yaml`, or `.yml`) are supported as
   overlays. Environment overrides use the `MIR2_CLIENT__...` prefix.
- The parsed settings are exposed via structured Rust types covering graphics,
   network, logging, audio, launcher, gameplay, chat visibility, and chat
   filters. Future subsystems will extend these sections instead of relying on
   global statics.

## Next steps

1. **Protocol parity** – port packet definitions from `Shared/ClientPackets.cs`
   and `Shared/ServerPackets.cs` into Rust modules, ensuring binary
   compatibility.
2. **Resource system** – translate the image/sound library loaders.
3. **Rendering** – implement a windowed renderer (likely wgpu or sdl2) with the
   Mir2 scene graph.
4. **UI framework** – rebuild MirControls in Rust, potentially leveraging an
   immediate-mode abstraction over the renderer.
5. **Input & configuration** – port key bindings and settings persistence.

Contributions are welcome; please keep the port modular so that we can swap
backends without touching game logic.
