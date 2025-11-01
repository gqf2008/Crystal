# Docs 目录清理计划

**日期**: 2025-01-01
**目标**: 删除过期、重复、临时文档，保留核心文档

---

## 📋 保留文档（核心/最新）

### 1. 架构文档（保留5个）
- ✅ `ECS_AUDIT_REPORT.md` - 最新的 ECS 架构审查报告（2024年，650行）
- ✅ `CAMERA_TRANSFORM_EXPLANATION.md` - 相机变换架构说明（最新）
- ✅ `RENDER_SYSTEMS_CLARIFICATION.md` - 渲染系统职责说明（最新）
- ✅ `UI_SYSTEM_GUIDE.md` - UI系统使用指南
- ✅ `项目架构说明.md` - 项目整体架构

### 2. 使用指南（保留3个）
- ✅ `ECS重构使用指南.md` - ECS 使用指南
- ✅ `SERVER_INTEGRATION_GUIDE.md` - 服务器集成指南
- ✅ `从C#_OOP到Rust_ECS迁移指南.md` - 迁移指南

### 3. 专题说明（保留5个）
- ✅ `COORDINATE_SYSTEM.md` - 坐标系统说明
- ✅ `事件清理机制说明.md` - 事件系统说明
- ✅ `视口裁剪架构说明.md` - 视口裁剪说明
- ✅ `寻路调试功能说明.md` - 寻路调试说明
- ✅ `GGEZ_Z_AXIS_说明.md` - Z轴说明

### 4. 子目录（保留2个）
- ✅ `scenes/` - 场景相关文档
- ✅ `testing/` - 测试相关文档

**保留总计**: 15 个文件 + 2 个目录

---

## 🗑️ 删除文档（过期/重复/临时）

### 分类1: 过期的架构文档（删除11个）
这些文档已被 `ECS_AUDIT_REPORT.md` 替代：

- ❌ `ARCHITECTURE_CORRECTION_2024.md` - 已过期
- ❌ `ARCHITECTURE_SUMMARY.md` - 已整合到审查报告
- ❌ `ECS_ARCHITECTURE.md` - 旧版架构说明
- ❌ `ECS_ARCHITECTURE_REVIEW.md` - 已被审查报告替代
- ❌ `ECS_IMPLEMENTATION_PLAN.md` - 实现计划已完成
- ❌ `ECS_REFACTOR_PLAN.md` - 重构计划已完成
- ❌ `ECS_RESOURCES_VS_COMPONENTS.md` - 已过期的设计讨论
- ❌ `ECS_SYSTEMS_DIRECTORY.md` - 已整合到代码中
- ❌ `ECS_SYSTEMS_USAGE.md` - 已整合到使用指南
- ❌ `ECS_SYSTEM_EVALUATION.md` - 评估已完成
- ❌ `OOP_vs_ECS_架构对比.md` - 已完成迁移

### 分类2: 临时修复报告（删除20个）
这些是临时的 bug 修复记录，问题已解决：

- ❌ `BASIC_FUNCTIONALITY_FIX.md`
- ❌ `CODE_AUDIT_REPORT.md`
- ❌ `CODE_FIX_REPORT.md`
- ❌ `CODE_REVIEW_REPORT.md`
- ❌ `CLEANUP_LOG.md`
- ❌ `DEPRECATED_CLEANUP_REPORT.md`
- ❌ `LAYER_CLEANUP_REPORT.md`
- ❌ `LOGIN_SCENE_FIX.md`
- ❌ `MAP_VIEWER_ECS_FIXES.md`
- ❌ `NETWORK_ERROR_FIX_REPORT.md`
- ❌ `NEW_ACCOUNT_FIX.md`
- ❌ `REFACTOR_AUDIT_REPORT.md`
- ❌ `REFACTOR_LOGIN_SCENE.md`
- ❌ `UI_DIALOG_FIXES.md`
- ❌ `VALIDATION_LOGIC_FIX.md`
- ❌ `LOGIN_SELECT_ECS_EVALUATION.md`
- ❌ `TEXTURE_RESOURCE_AUDIT.md`
- ❌ `全局事件系统重构审查报告.md`
- ❌ `bugfix/NewAccountDialog事件处理修复.md` - 整个目录删除

### 分类3: 完成报告（删除15个）
这些是阶段性完成报告，信息已过期：

- ❌ `ECS_MAGIC_LEARNING_COMPLETE.md`
- ❌ `ECS_MAP_VIEWER_BUGFIX.md`
- ❌ `ECS_MAP_VIEWER_FPS优化修复.md`
- ❌ `ECS_MAP_VIEWER_PERFORMANCE.md`
- ❌ `ECS_MAP_VIEWER_SUCCESS.md`
- ❌ `ECS_MAP_VIEWER_高级性能优化.md`
- ❌ `ECS_UI_REFACTOR_COMPLETE.md`
- ❌ `ECS模块迁移完成报告.md`
- ❌ `ECS系统提取进度-最新.md`
- ❌ `ECS系统提取进度.md`
- ❌ `GAME_SCENE_完成报告.md`
- ❌ `GGEZ_HECS_READY.md`
- ❌ `MONSTER_SYSTEM_COMPLETE.md`
- ❌ `MONSTER_SYSTEM_实现报告.md`
- ❌ `PLAYER_INITIALIZATION_COMPLETE.md`

### 分类4: 过期的性能/技术分析（删除5个）
- ❌ `GPU使用率分析报告.md`
- ❌ `InstanceArray_实现总结.md`
- ❌ `InstanceArray_性能优化分析.md`
- ❌ `DrawParam完整指南.md` - ggez 官方文档更准确

### 分类5: 重复/过期的专题文档（删除8个）
- ❌ `BUFF_SYSTEM.md` - 已整合到代码注释
- ❌ `CAMERA_SYSTEM.md` - 已被 CAMERA_TRANSFORM_EXPLANATION 替代
- ❌ `GAME_LOGIC_COMPARISON.md` - 过期对比
- ❌ `GAME_SCENE_ARCHITECTURE_REVIEW.md` - 已过期
- ❌ `GAME_SCENE_参考手册.md` - 信息过期
- ❌ `GAME_SCENE_核心功能迁移清单.md` - 迁移已完成
- ❌ `MAP_VIEWER_ECS_QUICKSTART.md` - 已被审查报告替代
- ❌ `MAP_VIEWER_V3.md` - 版本过期

### 分类6: 临时/杂项文档（删除5个）
- ❌ `NEW_FILES_CHECKLIST.md` - 临时清单
- ❌ `NEXT_STEPS.md` - 临时待办
- ❌ `RENDER_SYSTEMS_EXPLAINED.md` - 已被 CLARIFICATION 替代
- ❌ `UI_EVENT_DISPATCHER.md` - 已整合
- ❌ `UI_INTEGRATION_EXAMPLE.md` - 已整合到 GUIDE
- ❌ `UI_MIGRATION_SUMMARY.md` - 迁移已完成
- ❌ `network_architecture_v2.md` - 版本过期

**删除总计**: 64 个文件 + 1 个目录

---

## 📊 清理前后对比

| 项目 | 清理前 | 清理后 | 减少 |
|------|--------|--------|------|
| 文件数量 | 79 | 15 | -81% |
| 目录数量 | 3 | 2 | -33% |
| 估计总行数 | ~30,000 | ~5,000 | -83% |

---

## 🔄 清理后的目录结构

```
docs/
├── 📚 核心架构（5个）
│   ├── ECS_AUDIT_REPORT.md                    ⭐ 最新审查报告
│   ├── CAMERA_TRANSFORM_EXPLANATION.md        ⭐ 相机架构说明
│   ├── RENDER_SYSTEMS_CLARIFICATION.md        ⭐ 渲染系统说明
│   ├── UI_SYSTEM_GUIDE.md                     UI系统指南
│   └── 项目架构说明.md                         项目整体架构
│
├── 📖 使用指南（3个）
│   ├── ECS重构使用指南.md                      ECS使用指南
│   ├── SERVER_INTEGRATION_GUIDE.md             服务器集成
│   └── 从C#_OOP到Rust_ECS迁移指南.md          迁移指南
│
├── 📝 专题说明（5个）
│   ├── COORDINATE_SYSTEM.md                    坐标系统
│   ├── 事件清理机制说明.md                     事件系统
│   ├── 视口裁剪架构说明.md                     视口裁剪
│   ├── 寻路调试功能说明.md                     寻路调试
│   └── GGEZ_Z_AXIS_说明.md                     Z轴说明
│
└── 📁 子目录（2个）
    ├── scenes/                                 场景文档
    └── testing/                                测试文档
```

---

## ✅ 清理执行

执行以下 PowerShell 命令：

```powershell
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust\docs

# 删除过期架构文档（11个）
Remove-Item ARCHITECTURE_CORRECTION_2024.md
Remove-Item ARCHITECTURE_SUMMARY.md
Remove-Item ECS_ARCHITECTURE.md
Remove-Item ECS_ARCHITECTURE_REVIEW.md
Remove-Item ECS_IMPLEMENTATION_PLAN.md
Remove-Item ECS_REFACTOR_PLAN.md
Remove-Item ECS_RESOURCES_VS_COMPONENTS.md
Remove-Item ECS_SYSTEMS_DIRECTORY.md
Remove-Item ECS_SYSTEMS_USAGE.md
Remove-Item ECS_SYSTEM_EVALUATION.md
Remove-Item OOP_vs_ECS_架构对比.md

# 删除临时修复报告（19个）
Remove-Item BASIC_FUNCTIONALITY_FIX.md
Remove-Item CODE_AUDIT_REPORT.md
Remove-Item CODE_FIX_REPORT.md
Remove-Item CODE_REVIEW_REPORT.md
Remove-Item CLEANUP_LOG.md
Remove-Item DEPRECATED_CLEANUP_REPORT.md
Remove-Item LAYER_CLEANUP_REPORT.md
Remove-Item LOGIN_SCENE_FIX.md
Remove-Item MAP_VIEWER_ECS_FIXES.md
Remove-Item NETWORK_ERROR_FIX_REPORT.md
Remove-Item NEW_ACCOUNT_FIX.md
Remove-Item REFACTOR_AUDIT_REPORT.md
Remove-Item REFACTOR_LOGIN_SCENE.md
Remove-Item UI_DIALOG_FIXES.md
Remove-Item VALIDATION_LOGIC_FIX.md
Remove-Item LOGIN_SELECT_ECS_EVALUATION.md
Remove-Item TEXTURE_RESOURCE_AUDIT.md
Remove-Item 全局事件系统重构审查报告.md
Remove-Item -Recurse bugfix/

# 删除完成报告（15个）
Remove-Item ECS_MAGIC_LEARNING_COMPLETE.md
Remove-Item ECS_MAP_VIEWER_BUGFIX.md
Remove-Item ECS_MAP_VIEWER_FPS优化修复.md
Remove-Item ECS_MAP_VIEWER_PERFORMANCE.md
Remove-Item ECS_MAP_VIEWER_SUCCESS.md
Remove-Item ECS_MAP_VIEWER_高级性能优化.md
Remove-Item ECS_UI_REFACTOR_COMPLETE.md
Remove-Item ECS模块迁移完成报告.md
Remove-Item ECS系统提取进度-最新.md
Remove-Item ECS系统提取进度.md
Remove-Item GAME_SCENE_完成报告.md
Remove-Item GGEZ_HECS_READY.md
Remove-Item MONSTER_SYSTEM_COMPLETE.md
Remove-Item MONSTER_SYSTEM_实现报告.md
Remove-Item PLAYER_INITIALIZATION_COMPLETE.md

# 删除过期技术分析（4个）
Remove-Item GPU使用率分析报告.md
Remove-Item InstanceArray_实现总结.md
Remove-Item InstanceArray_性能优化分析.md
Remove-Item DrawParam完整指南.md

# 删除重复/过期专题（8个）
Remove-Item BUFF_SYSTEM.md
Remove-Item CAMERA_SYSTEM.md
Remove-Item GAME_LOGIC_COMPARISON.md
Remove-Item GAME_SCENE_ARCHITECTURE_REVIEW.md
Remove-Item GAME_SCENE_参考手册.md
Remove-Item GAME_SCENE_核心功能迁移清单.md
Remove-Item MAP_VIEWER_ECS_QUICKSTART.md
Remove-Item MAP_VIEWER_V3.md

# 删除临时/杂项（7个）
Remove-Item NEW_FILES_CHECKLIST.md
Remove-Item NEXT_STEPS.md
Remove-Item RENDER_SYSTEMS_EXPLAINED.md
Remove-Item UI_EVENT_DISPATCHER.md
Remove-Item UI_INTEGRATION_EXAMPLE.md
Remove-Item UI_MIGRATION_SUMMARY.md
Remove-Item network_architecture_v2.md

# 验证结果
Write-Host "清理完成！剩余文件："
Get-ChildItem -File | Select-Object Name
```

---

## 📌 注意事项

1. **备份**: 如果担心误删，可以先移动到 `docs/archive/` 目录
2. **Git 历史**: 删除的文档仍可通过 git 历史访问
3. **后续维护**: 新文档应该遵循"临时文档定期清理"原则

---

**清理理由**: 
- 减少文档维护负担
- 避免过期信息误导
- 提高关键文档可见性
- 降低新开发者学习曲线
