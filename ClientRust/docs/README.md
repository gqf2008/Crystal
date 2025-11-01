# 📚 ClientRust 文档目录

**最后更新**: 2025-01-01  
**文档数量**: 14 个核心文档 + 2 个子目录

---

## 📖 快速导航

### 🎯 新手入门（按顺序阅读）

1. **[项目架构说明](./项目架构说明.md)** - 从这里开始！
   - 项目整体结构
   - 技术栈介绍
   - 目录组织

2. **[ECS重构使用指南](./ECS重构使用指南.md)** - ECS 核心概念
   - Entity-Component-System 架构
   - 如何创建组件
   - 如何编写系统

3. **[从C#_OOP到Rust_ECS迁移指南](./从C#_OOP到Rust_ECS迁移指南.md)** - 给 C# 开发者
   - OOP vs ECS 思维转换
   - C# 代码如何迁移到 Rust
   - 常见模式对比

---

## 🏗️ 核心架构文档

### ⭐ 必读

- **[ECS_AUDIT_REPORT.md](./ECS_AUDIT_REPORT.md)** - **最重要！**
  - ECS 架构完整审查报告（650行）
  - 21个组件模块详解
  - 21个系统详解（优先级、职责、依赖关系）
  - 已知问题和改进建议
  - **推荐**: 开发前先阅读此文档！

### 🎥 渲染与相机

- **[CAMERA_TRANSFORM_EXPLANATION.md](./CAMERA_TRANSFORM_EXPLANATION.md)** - 相机架构说明
  - CameraSystem 的真实职责
  - 坐标变换在哪里执行
  - 为什么不是"相机矩阵系统"
  - 常见误解澄清

- **[RENDER_SYSTEMS_CLARIFICATION.md](./RENDER_SYSTEMS_CLARIFICATION.md)** - 渲染系统职责
  - EntityRenderSystem vs SpriteRenderSystem
  - 渲染系统对比表
  - 删除建议

- **[COORDINATE_SYSTEM.md](./COORDINATE_SYSTEM.md)** - 坐标系统
  - 世界坐标 vs 屏幕坐标
  - 坐标转换公式
  - 瓦片坐标系统

- **[视口裁剪架构说明.md](./视口裁剪架构说明.md)** - 视口裁剪
  - 视锥裁剪原理
  - 性能优化技巧

- **[GGEZ_Z_AXIS_说明.md](./GGEZ_Z_AXIS_说明.md)** - Z轴渲染
  - 深度排序原理
  - 渲染顺序控制

---

## 🎮 系统专题

### UI 系统

- **[UI_SYSTEM_GUIDE.md](./UI_SYSTEM_GUIDE.md)** - UI系统使用指南
  - UI对话框架构
  - 事件处理
  - 常见模式

### 事件系统

- **[事件清理机制说明.md](./事件清理机制说明.md)** - 事件生命周期
  - GlobalEvents 组件
  - 事件收集与清理
  - 为什么不需要 EventCleanupSystem

### 寻路系统

- **[寻路调试功能说明.md](./寻路调试功能说明.md)** - 寻路调试
  - 寻路算法说明
  - 调试可视化

---

## 🌐 服务器集成

- **[SERVER_INTEGRATION_GUIDE.md](./SERVER_INTEGRATION_GUIDE.md)** - 服务器集成指南
  - 网络架构
  - 客户端-服务器通信
  - 网络同步

---

## 📁 子目录

### [scenes/](./scenes/) - 场景相关文档
- 各个场景（登录、角色选择、游戏场景等）的详细说明

### [testing/](./testing/) - 测试相关文档
- 测试策略
- 测试用例

---

## 🗑️ 文档清理记录

**清理日期**: 2025-01-01  
**清理文档数**: 64 个过期/重复/临时文档  
**详细清单**: 见 [DOCS_CLEANUP_PLAN.md](./DOCS_CLEANUP_PLAN.md)

### 清理前后对比
- **清理前**: 79 个文件
- **清理后**: 14 个文件
- **减少**: 81%

### 删除的文档类型
1. 过期架构文档（11个）
2. 临时修复报告（19个）
3. 完成报告（15个）
4. 过期技术分析（4个）
5. 重复/过期专题（8个）
6. 临时/杂项（7个）

---

## 📝 文档维护规则

### 何时创建新文档

✅ **应该创建**：
- 新的核心架构设计
- 重要的系统使用指南
- 长期有效的技术说明

❌ **不应该创建**：
- 临时 bug 修复记录（使用 git commit message）
- 阶段性进度报告（使用 TODO 列表）
- 重复已有文档的内容

### 文档命名规范

- 英文文档：`UPPERCASE_SNAKE_CASE.md`（如：`ECS_AUDIT_REPORT.md`）
- 中文文档：`中文下划线.md`（如：`项目架构说明.md`）
- 专题文档：`主题_子主题.md`（如：`CAMERA_TRANSFORM_EXPLANATION.md`）

### 定期清理

每个季度审查一次文档：
- 删除过期的临时文档
- 合并重复内容
- 更新核心文档

---

## ❓ 常见问题

### Q: 我需要了解 ECS 架构，从哪里开始？
**A**: 按顺序阅读：
1. `项目架构说明.md` - 了解整体结构
2. `ECS重构使用指南.md` - 学习 ECS 概念
3. `ECS_AUDIT_REPORT.md` - 深入了解所有组件和系统

### Q: 我在写渲染相关代码，需要看哪些文档？
**A**: 阅读渲染专题：
1. `CAMERA_TRANSFORM_EXPLANATION.md` - 相机架构
2. `RENDER_SYSTEMS_CLARIFICATION.md` - 渲染系统职责
3. `COORDINATE_SYSTEM.md` - 坐标系统
4. `视口裁剪架构说明.md` - 性能优化

### Q: 原来的 XXX.md 文档去哪了？
**A**: 如果是过期文档，已被删除。可以：
1. 查看 `DOCS_CLEANUP_PLAN.md` 了解删除原因
2. 通过 `git log` 查看历史版本
3. 相关内容可能已整合到其他核心文档中

### Q: 我想添加新文档，需要遵循什么规则？
**A**: 
1. 确认不与现有文档重复
2. 确认是长期有效的内容（非临时记录）
3. 遵循命名规范
4. 添加到本 README 的相应分类中

---

## 🔗 外部资源

- [hecs 文档](https://docs.rs/hecs/) - ECS 库官方文档
- [ggez 文档](https://docs.rs/ggez/) - 游戏引擎官方文档
- [Rust Book](https://doc.rust-lang.org/book/) - Rust 语言学习

---

**维护者**: 开发团队  
**反馈**: 如发现文档问题或有改进建议，请提交 Issue
