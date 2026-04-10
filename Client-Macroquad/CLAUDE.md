# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Legend of Mir 2 (传奇2)** game client written in Rust, built on the **macroquad** game engine. It uses an **ECS architecture** with **hecs** and a **state-machine scene system** with `enum_dispatch`.

## Essential Commands

### Build & Run
```bash
# Main game (full flow: Login → Character Select → Game)
cargo run --bin mir2
cargo run --bin mir2 --release   # optimized

# Test binaries for specific scenes
cargo run --bin test_login       # Login scene
cargo run --bin test_select      # Character select
cargo run --bin test_game_scene  # Game scene with ECS
cargo run --bin map_viewer       # Map viewer tool
cargo run --bin test_main_dialog # Main dialog UI
cargo run --bin test_belt_dialog # Belt/quickbar dialog
cargo run --bin test_inventory_hybrid  # Inventory dialog
```

### Configuration
- `config.ini` - Runtime network/config (UseMock, ServerAddr, etc.)
- Game assets must be present in `Data/` directory (`.Lib` format)

### Running Tests
There is no `cargo test` suite. Testing is done via standalone binaries in `src/bin/`. A few `#[cfg(test)]` modules exist in scattered source files.

## Architecture

### High-Level Structure

```
src/
├── lib.rs                  # Library root - exports all modules
├── bin/                    # Multiple binary entry points
├── game.rs                 # GameState + GameContext (central runtime)
├── components/             # ECS components (hecs)
├── systems/                # ECS systems (layered by priority)
│   ├── input/              # PlayerControl, AutoPotion, LocalPlayerAi
│   ├── infra/              # Network, MapBootstrap, MapLoad
│   ├── logic/              # Combat, Physics (Movement/Collision/Pathfinding), AI
│   ├── presentation/       # Animation, Camera, Particle, Dialog
│   └── rendering/          # SpriteRender, UIRender, EffectRender
├── scenes/                 # Scene system (enum_dispatch)
│   ├── login_scene.rs
│   ├── select_scene.rs
│   ├── game_scene.rs
│   └── dialogs/game/       # egui-based in-game dialogs
├── network/                # TCP client + mock + protocol handlers
├── resources/              # MLibrary parser, resource manager
├── map_renderer/           # Mesh-based map renderer
├── event_bus/              # Typed event bus (5 event types)
├── camera/                 # 2D camera system
├── ui/                     # UI utilities
├── objects/                # Object frame data (animations)
└── coord.rs                # Coordinate system utilities
```

### Scene System

Uses `enum_dispatch` for zero-cost dynamic dispatch:

```rust
Scene trait (enum_dispatch)
  ├── LoginScene
  ├── SelectScene
  ├── GameScene
  └── LoadingScene

// SceneKind enum auto-generates match-based dispatch
// Transitions via SceneTransition enum
```

`GameState` in `game.rs` manages the current scene and transitions. Each frame: `handle_input()` → `update()` → `render()`.

### ECS System (hecs)

Custom `SystemScheduler` with two traits:
- `LogicSystem` - pure logic (update only)
- `RenderSystem` - rendering (draw, with optional update)

**Priority layers** (executed in order each frame):
- `0-99`: Infrastructure (resource preload, scene, save)
- `100-199`: Input & Network
- `200-599`: Game Logic (AI, Combat, Pathfinding, Movement, Collision)
- `600-899`: Presentation (Animation, Particle, Sound, Camera, UI, Dialog)
- `900-1999`: Rendering (Map, Sprite, Entity, Effect, UI Render, Lighting, Text)
- `9000+`: Debug tools

### Event Bus

Typed `EventBus` with 5 separate queues: `InputEvent`, `NetworkEvent`, `GameLogicEvent`, `UIEvent`, `PresentationEvent`. Frame-scoped (cleared at frame end).

### UI Architecture

Two UI frameworks coexist:
1. **macroquad::ui (megaui)** - Login/select scenes
2. **egui** - In-game dialogs (MainDialog, InventoryDialog, CharacterDialog, etc.)

UI focus management uses 3 layers: Foreground > Middle > Background.

### Networking

Two modes via `config.ini`:
- **Mock mode** - Offline testing with simulated responses
- **Real TCP mode** - Connects to game server

Shared protocol/types come from `mir2_shared` (sibling repo at `../SharedRust`).

### Key Dependencies

| Purpose | Crate |
|---|---|
| Game Engine | `macroquad 0.4` (audio feature) |
| ECS | `hecs 0.10` + custom `ecs_macros` |
| Scene Dispatch | `enum_dispatch 0.3` |
| Shared Protocol | `mir2_shared` (path: `../SharedRust`) |
| Async | `tokio` (optional), `crossbeam-channel` |
| Logging | `tracing` + `tracing-subscriber` |

Patches `miniquad` to use local fork at `miniquad/` (IME support).

### Cargo Features

- `ecs_rendering` - Experimental ECS-based rendering (off by default)
- `tokio` - Async runtime (off by default)
- `backend-ggez` - Legacy compatibility (off by default)

## Important Notes

- **Game assets**: The `Data/` directory must contain `.Lib` game asset files. Without them, most binaries will fail to load.
- **`config.ini`**: Controls mock vs real network mode. Set `UseMock=true` for offline testing.
- **Release profile**: LTO + strip enabled. Development profile has opt-level=1.
- **Documentation**: Extensive Chinese docs in `docs/` directory. `GAMESCENE_UI_TODO.md` tracks UI implementation status (600+ lines, 6 phases).
