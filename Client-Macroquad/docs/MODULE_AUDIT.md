# Client-Macroquad 模块使用审查（Module Audit）

日期：2025-12-13 | 更新：2026-04-26 (所有模块已接入，无未使用遗留)

## 目标

- 盘点 `src/` 下各模块/子目录：
  - 是否进入编译树（被 `lib.rs` 或某个 `mod.rs` 引入）
  - 是否被运行链路实际使用（从 `src/bin/*.rs` 的入口往下）
- 输出“可读性优先”的结论：哪些是当前真实依赖、哪些是未接入/历史遗留、哪些建议收敛。

## 方法

1. 从入口程序开始：`src/bin/*.rs`（例如 `test_game_scene`, `scene_demo`）
2. 追踪其 `use client_macroquad::...` 引用到的库模块。
3. 对 `src/` 下顶层目录做两层判断：
   - **编译树层**：是否在 `src/lib.rs` 中 `pub mod ...`
   - **运行层**：是否存在明确的调用链/实例化/执行点

> 注意：Rust 的“被编译”不等于“被运行”。像 `systems/` 这种即使编译进库，如果没有调度器入口调用，它对实际功能也没有贡献，但会显著增加阅读负担。

## 结论摘要（优先级从高到低）

### P0：明确未接入/死代码风险（建议尽快处理）

1. `src/input_support/`
   - 现状：不在 `src/lib.rs` 的模块列表中，因此**不在编译树**；目录内文件仅彼此引用。
   - 风险：目录看起来“像正式功能（IME/组合输入）”，但实际不会被任何运行路径使用，误导阅读。
   - 建议：
     - 要么接入：在 `src/lib.rs` 增加 `pub mod input_support;` 并提供明确使用点；
     - 要么移除/转移到 `docs/` 或 `experiments/`，并在 README 标注。

2. `src/systems/`（ECS Systems）
   - 现状：模块被编译进库，但没有发现任何运行入口创建/使用 `SystemScheduler`。
   - 影响：系统数量多、目录深，会给人“ECS 已经在跑”的错觉。
   - 建议：在模块总览文档/注释中明确：当前仍未接入主循环；或用 feature gate 把系统群隔离。

### P1：重复实现/概念分裂（建议收敛）

1. IME 支持的分裂
   - `src/platform/ime.rs`：通过 Windows API/imm32 设置 IME 位置（`set_ime_position(i32, i32)`）。
   - `src/utils/ime.rs`：另一套 Windows IME 位置设置（`set_ime_position(f32, f32)`）。
   - 实际使用：聊天输入中使用的是 `miniquad::window::set_ime_enabled/set_ime_position`。
   - 建议：选一种作为“唯一入口”。例如：
     - 方案 A：统一走 `miniquad::window::*`（最贴近 macroquad 生态）；
     - 方案 B：统一走 `platform::ime` 并在 chat 输入中改为调用它；
     - 同时删除/合并另一份重复实现。

2. 自定义相机模块 `src/camera/`
   - 现状：存在 `GameCamera2D` 等封装，但运行链路里主要直接用 macroquad 的 `Camera2D`。
   - 建议：
     - 如果短期不用，标注为 experimental 或移出主路径；
     - 如果要用，明确哪些场景/渲染路径应该改用 `GameCamera2D`。

### P2：编译了但当前运行链路依赖不强（可后续再决定）

1. `src/event_bus/`
   - 现状：`GameContext` 持有 `EventBus`，模块内部实现完整，还带少量测试/示例。
   - 但：暂未发现主运行链路中实际 `send_input/send_logic` 的调用点。
   - 建议：
     - 如果计划用 EventBus 作为输入/逻辑的解耦方式，尽快选一个最小切入点接入；
     - 否则将其标注为“预备架构”，避免被误认为已全面使用。

2. `src/network/`
   - 现状：存在 `NetworkBuilder/NetContext`、handlers、mock client 等较完整结构。
   - 但：`GameContext` 当前使用的是极简 `components::network::NetworkContext`（仅 `connected: bool`），未看到实际连接/网络循环接入场景。
   - 建议：同上：要么尽快接入最小链路，要么明确为未接入。

## 模块概览（按 src 顶层目录）

| 模块/目录 | 是否在编译树 | 是否在运行链路使用 | 备注 |
|---|---:|---:|---|
| `camera/` | 是 | 否（当前未发现引用点） | 自定义相机封装未被场景使用 |
| `components/` | 是 | 部分 | 大量组件主要服务 `systems/`，但系统未接入调度 |
| `core/` | 是 | 是 | 基础错误/常量/设置等 |
| `event_bus/` | 是 | 弱 | 当前更像“预备架构”，主链路调用少 |
| `input_support/` | 否 | 否 | 目录孤岛，不会被编译 |
| `map_renderer/` | 是 | 是 | 地图渲染已被场景/工具使用 |
| `network/` | 是 | 弱 | 结构较全但未接入主链路 |
| `platform/` | 是 | 否（当前未发现调用点） | Windows IME API 封装 |
| `resources/` | 是 | 是 | 资源加载/地图读取等核心路径 |
| `scenes/` | 是 | 是 | 当前主运行逻辑都在这里 |
| `systems/` | 是 | 否（无调度入口） | “写了但不执行”的最大噪音源 |
| `ui/` | 是 | 是 | 原生 UI 渲染/对话框 |
| `utils/` | 是 | 否（当前未发现调用点） | IME 重复实现所在 |

## 建议的后续动作（不改变架构的前提下）

- 最小化误导：
  - 在 `systems/` 和 `input_support/` 的模块头注释写明“当前未接入运行主循环”。
- 目录收敛：
  - IME：在 `platform/ime`、`utils/ime`、`miniquad::window` 三者中收敛到单一入口。
- 技术债记录：
  - 对 `systems/rendering/map_system.rs` 这类明显未完成的系统实现，标注 TODO 与状态（避免被当作可用功能）。

---

如果你希望下一步继续推进（仍然按 A 的策略：先审查/不重构），建议从“给 systems 写一个最小接入入口”作为单点突破：哪怕只接入 `InputStateSystem` + `PlayerControlSystem` + `MovementSystem`，也能让 `systems/` 从“死目录”变成“可演进目录”。
