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

✅ 登录/选角界面完整移植 + 呈现修复（里程碑 6b）：
- 登录：全屏 ChrSel[0] 背景 + 居中 328x220 对话框（Prguse[1084]）
  + Title[30] logo + Title[31/32] 标签 + Title[320-328] 三帧图按钮（坐标对齐原版）
- 启动动画：登录成功后播 ChrSel 0-18 帧动画 → 进入选角
- 选角：ChrSel 动画背景 + 角色预览（ChrSel[base_index+frame]，16帧/0.25s）
- 关键修复：强制 DX12 渲染后端（Vulkan swapchain present 在此机器冻结
  → 窗口只显示第一帧；DX12 实时呈现正常）
- 中文字体内嵌（Font::from_bytes），不依赖 assets 路径

✅ UI 改用 Bevy 内置 bevy_ui（里程碑 6a）：
- 移除 bevy_egui 依赖，登录/选角界面改为 bevy_ui（Node/Button/Text/Interaction）
- 中文字体：FontSource::Handle 加载阿里普惠体（bevy_ui 的 TextFont）
- 自定义文本输入框（TextInputNode + MessageReader<KeyboardInput>，支持中文/退格/密码打码）
- 键盘输入用 Bevy 0.19 MessageReader（Event 已更名 Message）

✅ 网络层 mock 模式（里程碑 5）：
- 协议管道：codec(长度前缀+XOR) → PacketHeader → 类型化包（mir2_shared）
- Mock 服务器线程：Login → LoginSuccess(2角色) → StartGame → MapChanged+Object 包 → KeepAlive
- NetworkSystem 分发：LoginSuccess→Select、MapChanged→加载地图、ObjectPlayer/Monster/Npc→生成角色
- 角色选择界面（Select 场景）：角色列表 + 进入游戏
- 本地玩家 ghost/遮挡/动画沿用；demo 角色改由 --demo 提供，默认走网络对象
- 用法：登录界面点"登录"；--auto-enter 自动驱动 mock 流程；--demo 演示角色

✅ 场景系统 + bevy_egui 登录界面 + 事件总线（里程碑 4）：
- AppState 状态机：Login（登录界面）→ Game（游戏场景）
- bevy_egui 0.41 登录界面（中文字体、账号/密码、进入游戏按钮）
- 事件总线：Bevy 0.19 Observer + trigger 模式（GameEvent）
- 地图/角色按状态加载：OnEnter(Game) 才加载，登录界面轻量
- 用法：cargo run（登录界面）；--auto-enter 自动进游戏；--no-actors 纯地图

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
