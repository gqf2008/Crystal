# Client-Bevy — 传奇2 (Legend of Mir 2) 客户端 Bevy 移植版

把 `Client-Macroquad`（macroquad + hecs 实现，~99% 完成）迁移到 **Bevy 0.19**。

## 当前状态（里程碑 1）

✅ 数据层移植（引擎无关）：
- `resources/mlibrary.rs` — `.Lib` 图像库解析（原始 RGBA，无 macroquad 耦合）
- `resources/map_reader.rs` — `.map` 解析（7 种格式，原样复用）
- `resources/libraries.rs` — 库注册表（MapLibs[0-399] 全部映射 + 24 个单体库）

✅ 渲染层：
- `map_renderer.rs` — 每 32x32 格合成一张块纹理（1536x1024），Back/Middle/Front 三层
- Bevy `Image` 资产 + Sprite 渲染，相机控制（WASD/方向键平移，+/- 缩放）

✅ Y 轴深度排序 + 前景遮挡（里程碑 3）：
- Front 层改为逐瓦片精灵，z = depth_y(格子底边)，角色 z = depth_y(脚底)
- 经典传奇遮挡：角色脚底 Y < 瓦片基准 → 被建筑/树遮挡；反之站在建筑前
- 唯一贴图去重，back/middle 保持分块渲染

✅ 角色/NPC/怪物精灵渲染与帧动画（里程碑 2）：
- `objects/frames.rs` — 帧表原样复用（PLAYER/MONSTER/NPC + 特殊怪物）
- `actor.rs` — ActorPlugin：ActorAnim/ActorAppearance/SpriteLayer 组件
- 帧号公式与 C#/macroquad 一致：DrawFrame = Start + Dir*OffSet + FrameIndex
- 分层渲染（护甲/发型/武器/特效）+ 精灵图缓存（按 库+槽位+帧 复用 Image 资产）
- 数组库懒加载（CArmour/CHair/CWeapon/CHumEffect/Monster/NPC）
- 演示：玩家绕方块行走、怪物待机转向/周期攻击、NPC 待机

🚧 待办（后续里程碑）：
- 场景系统（login/select/game）+ bevy_egui 对话框
- 网络层（17 handler + mock 模式）
- ECS 系统移植（combat/AI/physics/presentation）

## 运行

```bash
# 需要先有数据目录（共享 Client-Macroquad/Data，自动解析）
cargo run --bin client_bevy                  # 默认地图 0100
cargo run --bin client_bevy -- --map 11yearvilliage
cargo run --bin client_bevy -- --map n0
```

## 与 macroquad 版的关系

- 共享 `SharedRust`（协议）与游戏数据目录（`Client-Macroquad/Data`、`Client-Macroquad/Map`）
- 数据解析逻辑保持与 `Client-Macroquad/src/resources/*` 一致，仅去掉渲染引擎耦合

## 版本

当前锁定 `bevy = "0.19"`（ECS/渲染有大量新特性）。从 0.16 升级时仅需适配：
`Projection`/`OrthographicProjection` 移到 `bevy::camera`、`RenderAssetUsages` 移到 `bevy::asset`、
`WindowResolution` 接受 `(u32, u32)`。
