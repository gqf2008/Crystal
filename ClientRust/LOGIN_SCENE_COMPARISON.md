# LoginScene 模块关系说明

## 🔍 两个模块的关系

### 📍 位置对比

| 位置 | 模块 | 状态 | 用途 |
|------|------|------|------|
| `src/bevy/scenes/login_scene.rs` | **login_scene (v1)** | ⚠️ 已过时 | 早期实现,功能不完整 |
| `src/bevy/scenes/login_scene_v2/` | **login_scene_v2 (v2)** | ✅ **当前使用** | 完整重构版本,已投入使用 |

### 📊 代码量对比

```
login_scene.rs              669 行 (v1 - 基础实现)
login_scene_v2/mod.rs     2345 行 (v2 - 完整实现)
├─ components.rs           204 行 (组件定义)
├─ button_systems.rs       125 行 (按钮交互)
├─ input_systems.rs        316 行 (文本输入)
└─ ui_helpers.rs             1 行 (预留)

总计: v2 版本比 v1 多 3.5 倍的代码
```

### 🎯 功能对比

#### **login_scene.rs (v1)** - 基础版本
- ❌ 不完整的对话框系统
- ❌ 简单的按钮处理
- ❌ 缺少验证逻辑
- ❌ 未模块化
- ❌ **不在使用**

#### **login_scene_v2/ (v2)** - 完整版本 ✅
- ✅ 完整的新建账号对话框
- ✅ 完整的修改密码对话框
- ✅ 完整的按钮系统(悬停、点击、按压)
- ✅ 完整的文本输入系统(验证、光标闪烁)
- ✅ 完整的动画系统(19帧循环)
- ✅ 模块化结构(components, systems分离)
- ✅ **已投入使用**

---

## 📌 当前代码状态

### ✅ 模块导入配置

**src/bevy/scenes/mod.rs**:
```rust
pub mod login_scene;      // 仍保留(向后兼容)
pub mod login_scene_v2;   // 新版本

// 使用 v2 版本(完整功能实现)
pub use login_scene_v2::*;  // ⭐ 导出 v2,隐藏 v1
```

**src/bin/main_bevy.rs**:
```rust
// 引入 LoginScene V2 (完整功能版本)
use bevy::scenes::{
    setup_login_scene,
    cleanup_login_scene,
    // ... v2 系统
};
```

### ✅ 系统初始化

```rust
// 进入登录状态时设置 LoginScene (使用v2)
app.add_systems(OnEnter(GameState::Login), setup_login_scene);

// 退出登录状态时清理 LoginScene
app.add_systems(OnExit(GameState::Login), cleanup_login_scene);
```

---

## 🚀 结论

| 问题 | 答案 |
|------|------|
| **使用哪一个?** | 👉 **login_scene_v2** (v2版本) |
| **v1的目的是什么?** | 早期原型,已被v2完全替代 |
| **v1能删除吗?** | 可以,但目前保留作为备份 |
| **为什么保留v1?** | 代码审计、参考、向后兼容 |
| **v2是否完成?** | ✅ 是,但有一个已知的按钮位置偏差(暂搁置) |
| **接下来怎么做?** | 🎯 集成网络模块到login_scene_v2中 |

---

## 📝 建议

### 保留 login_scene.rs (v1) 的原因:
1. 代码对比参考
2. 向后兼容性
3. 作为设计文档

### 使用 login_scene_v2/ 的理由:
1. ✅ 功能更完整
2. ✅ 代码已模块化
3. ✅ 系统已优化
4. ✅ 已投入生产

### 后续优化:
- [ ] 可以将 login_scene.rs (v1) 移到 `_deprecated/` 文件夹
- [ ] 整理文档说明版本迁移过程
- [ ] 在网络集成后删除v1

---

## 📂 文件结构(建议)

```
src/bevy/scenes/
├── mod.rs                      # 场景模块入口
├── login_scene.rs              # ⚠️ v1 (已过时,可删除)
├── login_scene_v2/             # ✅ v2 (当前使用)
│   ├── mod.rs                  # 主场景逻辑
│   ├── components.rs           # 组件定义
│   ├── button_systems.rs       # 按钮系统
│   ├── input_systems.rs        # 输入系统
│   └── ui_helpers.rs           # UI辅助函数
├── select_scene/               # 📋 待实现
└── game_scene/                 # 🎮 待实现
```
