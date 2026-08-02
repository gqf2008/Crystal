# Client-Bevy 迁移计划（Bevy 0.19）

> 生成日期: 2026-08-02 | 分支: feat/bevy-client
> 目标: 将 `Client-Macroquad`（macroquad + hecs，约 99% 完成）迁移到 **Bevy 0.19**
> 参考: `Client-Macroquad/IMPLEMENTATION_PLAN.md`（P0–P5）、`ServerRust/docs/PORT_STATUS.md`
> 共享: `SharedRust`（协议库，276 S→C + 145 C→S）、`Client-Macroquad/Data`（游戏数据）

---

## 一、现状（已完成里程碑 M1–M6）

| 里程碑 | 内容 | 状态 |
|---|---|---|
| M1 | 地图渲染（.map 7 种格式、块纹理三层、相机控制） | ✅ |
| M2 | 角色/NPC/怪物精灵渲染与帧动画（帧表复用、精灵图缓存） | ✅ |
| M3 | Y 轴深度排序 + 前景遮挡（逐瓦片 front 层 + 角色 ghost 半透明） | ✅ |
| M4 | 场景系统（Intro/Login/Select/Game 状态机）+ 事件总线 + 登录界面 | ✅ |
| M5 | 网络层 mock 模式（codec 长度前缀+XOR → 类型化包 → 登录/选角/进游戏/对象生成） | ✅ |
| M6 | UI 迁移到 bevy_ui + 登录/选角/新建角色/删除确认完整移植 + IME 中文输入 + DX12 渲染后端 | ✅ |

当前 `cargo check` 通过；已接入包：LoginSuccess / StartGame / NewCharacter(Success) / DeleteCharacterSuccess / MapChanged / ObjectPlayer / ObjectMonster / ObjectNpc / ObjectRemove / KeepAlive。

## 二、待迁移代码量（对照 Client-Macroquad/src）

| 模块 | 文件数 | 行数 | Bevy 现状 |
|---|---:|---:|---|
| systems（infra/input/logic/presentation/rendering） | 61 | ~19,645 | 未移植（仅 network_system 雏形） |
| scenes/dialogs（56 个游戏对话框） | 57 | ~21,569 | 仅 login/select 相关已移植 |
| scenes（登录/选角/游戏场景） | 69 | ~24,572 | 状态机 + 登录/选角已移植 |
| network（17 handler + client + mock） | 32 | ~11,160 | codec + mock + 10 个包分支 |
| components（21 个 ECS 组件） | 21 | ~3,955 | 未移植 |
| event_bus（5 队列） | 5 | ~1,751 | 简易版（仅 LoginSuccess） |

核心大文件：network_apply_system（~240k，264 个 opcode 分支）、main_dialog（105k）、rendering/ui_system（~97k）、player_control（59k）、sprite_system/character（51k）、npc_goods_dialog（45k）、chat_dialog（44k）。

---

## 三、剩余里程碑提案（M7–M12）

### M7: 真实网络层（TCP 接入 ServerRust + 全量 handler）
- [ ] TCP 客户端线程（crossbeam 通道 + 阻塞读写），连接 `ServerRust`（默认 0.0.0.0:7000），ClientVersion 握手 + KeepAlive 心跳
- [ ] 移植 17 个 handler：connection / player / movement / chat / combat / item / npc / quest / group / trade / guild / mail / market / friend / hero / creature / social / ui_events
- [ ] NetworkContext 扩展为完整网络事件分发（对齐 macroquad 264 个 NetworkEvent 变体）
- [ ] 发包补齐：~145 个 C→S 包（移动/攻击/技能/对话框动作/组队/行会/邮件等）
- [ ] 断线重连、登录失败提示、KeepAlive 超时处理
- **验收**: 与真实 ServerRust 联调完成 登录→选角→进游戏→对象生成；`cargo check` / `clippy` 零 error

### M8: 游戏场景基础（HUD + 玩家控制）
- [ ] Game 场景骨架：StartGame/MapChanged 加载地图、相机跟随、出生点
- [ ] 主对话框（血/蓝/经验/金币/快捷栏/功能按钮）与聊天对话框（输入/系统消息/公告）
- [ ] 小地图、菜单/帮助/设置入口
- [ ] 玩家控制：点击移动（寻路）、攻击目标、NPC 交互、拾取、自动喝药
- [ ] 移植基础组件：player / movement / input / network / session / settings
- **验收**: 可进入游戏移动、聊天、看到怪物/NPC 待机与周围玩家

### M9: 对话框系统（56 个游戏对话框，分 4 批）
- [ ] 通用 UI 基建：对话框管理器（开关/焦点/拖拽/层级）、AmountBox、TextInput、native_ui_utils
- [ ] 第 1 批（核心）: inventory / character / belt / menu / minimap / chat_control / compass
- [ ] 第 2 批（交互）: npc / npc_goods / trade / amount_box / group / quest_log / friend / inspect
- [ ] 第 3 批（社交）: guild / guild_territory / mail / trust_merchant / item_rental / ranking / report / mentor / relationship / hero / intelligent_creature / mount
- [ ] 第 4 批（系统）: game_shop / refine / craft / socket / dura_status / npc_drop / roll / npc_awake / notice / chat_notice / timer / option / help / keyboard_layout / big_map / fishing / buff
- **验收**: 每个对话框交互与原版 C# / macroquad 一致（数据驱动，mock 先行）

### M10: 战斗 / 逻辑 / 物理
- [ ] combat: attack / magic / skill / buff / regen（公式与 C# 一致，91 法术差异化）
- [ ] decision: monster_ai（212 怪物行为数据驱动）/ npc_ai / npc_dialogue
- [ ] physics: movement / collision / pathfinding / map_load / map_update
- [ ] input: player_control / local_player_ai / auto_potion
- **验收**: 打怪、技能、怪物 AI 行为与 macroquad 版一致

### M11: 呈现与特效
- [ ] animation_system（帧动画/挂点/武器特效/坐骑状态）
- [ ] particle / weather / floating_text / health_bar_anim / sound（bevy_audio）
- [ ] rendering: sprite_system 分层遮挡 / effect_system / map_system 分块 + 唯一贴图去重
- [ ] 日夜循环、屏幕通知（ChatNotice）
- **验收**: 天气/粒子/音效/伤害飘字/血条/特效完整呈现

### M12: 打磨与验收
- [ ] config.ini 配置（账号保存/分辨率/音量/键位）、设置对话框
- [ ] 单元测试 + E2E（mock 网络驱动登录→选角→游戏→战斗），对齐 macroquad 10 E2E
- [ ] clippy 零 warning、release 构建、性能基线（精灵 Atlas/批处理）
- [ ] 与 ServerRust 全流程联调、README/文档更新
- **验收**: `cargo test --lib` 全过、`cargo build --release` 成功、10 E2E 通过

---

## 四、执行顺序与依赖

```
M7（真实网络）→ M8（HUD+控制）→ M9（对话框 1→4 批）→ M10（战斗/逻辑）→ M11（特效）→ M12（打磨）
```

- M9 第 1 批可与 M8 并行；M10 依赖 M8 的玩家控制
- 每个里程碑独立可编译、可运行（mock 模式保底）
- 对话框/系统移植时以 `Client-Macroquad/src` 为参考，去掉 macroquad 耦合，Bevy 用 ECS 组件 + System + bevy_ui

## 五、风险与已解决问题

| 风险 | 状态 |
|---|---|
| bevy_ui 布局与原版像素级对齐 | 登录/选角已验证（锚点左上 + 精灵坐标） |
| 中文输入（IME） | 已解决（Font::from_bytes + MessageReader） |
| 渲染后端冻结 | 强制 DX12（Vulkan present 在此机器异常） |
| 大量精灵性能 | 精灵图缓存已建；后续 Atlas/批处理 |
| 数据依赖 | 复用 Client-Macroquad/Data，`resolve_data_path` 自动解析 |
