# systems/ 分层架构

本目录包含 ECS 系统，按优先级+分层目录组织。

## 6 层架构

| 层 | 目录 | 优先级 | 职责 |
|----|------|--------|------|
| 0 | `infra/` | 0-99 | 资源加载、场景管理、网络底层 |
| 1 | `input/` | 100-199 | 输入采样、玩家控制 |
| 2 | `logic/` | 200-599 | AI决策、战斗、物理、移动 |
| 3 | `presentation/` | 600-899 | 动画、相机、粒子、UI表现 |
| 4 | `rendering/` | 900-1999 | 精灵/特效/地图/UI渲染 |
| 5 | `dbug/` | 9000+ | 调试工具 |

## 系统类型

- `LogicSystem`: `update(&mut GameContext, f32) -> GameResult`
- `RenderSystem`: `update()` + `draw()` 方法

## 调度顺序

- `SystemScheduler::update()` — 按 priority 升序执行所有系统的 update
- `SystemScheduler::draw()` — 按 priority 升序执行 RenderSystem 的 draw

## 约定

1. 优先级使用 `systems::priority::*` 常量
2. 系统间通过组件/资源通信
3. 迁移中系统需标注现状
