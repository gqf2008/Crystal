# 📚 SharedRust 文档导航

欢迎使用 SharedRust! 根据您的需求选择合适的文档：

## 🎯 我想...

### 快速开始使用
👉 **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** - 快速参考和常用代码片段
- ✅ 5 分钟快速上手
- ✅ 常用代码示例
- ✅ 速查表

### 了解模块对应关系
👉 **[PORTING_GUIDE.md](PORTING_GUIDE.md)** - 完整的 C# → Rust 移植指南
- ✅ 详细的模块映射表
- ✅ C# 和 Rust 代码对照
- ✅ 命名约定转换规则
- ✅ 使用示例

### 了解项目概况
👉 **[README.md](README.md)** - 项目总览
- ✅ 项目结构
- ✅ 主要特性
- ✅ 移植进度
- ✅ 快速开始

### 查看移植进度和技术细节
👉 **[MIGRATION_REPORT.md](MIGRATION_REPORT.md)** - 移植报告
- ✅ 详细的移植清单
- ✅ 测试结果
- ✅ 技术决策说明

### 查看 API 文档
👉 运行 `cargo doc --open`
- ✅ 完整的 API 文档
- ✅ 函数签名和说明
- ✅ 使用示例

---

## 📖 文档概览

| 文档 | 适合人群 | 主要内容 | 推荐度 |
|-----|---------|---------|--------|
| **[QUICK_REFERENCE.md](QUICK_REFERENCE.md)** | 所有开发者 | 快速参考、代码片段 | ⭐⭐⭐ |
| **[PORTING_GUIDE.md](PORTING_GUIDE.md)** | 从 C# 迁移的开发者 | 详细模块映射 | ⭐⭐⭐ |
| **[README.md](README.md)** | 新手 | 项目介绍、快速开始 | ⭐⭐⭐ |
| **[MIGRATION_REPORT.md](MIGRATION_REPORT.md)** | 技术人员 | 移植细节、进度 | ⭐⭐ |
| **API Docs** | 需要详细 API 的开发者 | 函数签名、参数说明 | ⭐⭐ |

---

## 🔍 按功能查找

### 数据包 (Packets)
- **C# → Rust 映射**: [PORTING_GUIDE.md § 3. 数据包系统](PORTING_GUIDE.md#3-数据包系统-packets)
- **快速使用**: [QUICK_REFERENCE.md § 3. 数据包操作](QUICK_REFERENCE.md#3-数据包操作)
- **已实现列表**: [PORTING_GUIDE.md § 3.2 客户端数据包](PORTING_GUIDE.md#32-客户端数据包-clientpackets)

### 方向和几何计算
- **C# → Rust 映射**: [PORTING_GUIDE.md § 5.1 方向和几何函数](PORTING_GUIDE.md#51-方向和几何函数)
- **快速使用**: [QUICK_REFERENCE.md § 2. 方向计算](QUICK_REFERENCE.md#2-方向计算)
- **API 文档**: `SharedRust/src/utils/direction.rs`

### Point 操作
- **C# → Rust 映射**: [PORTING_GUIDE.md § 5.2 Point 扩展方法](PORTING_GUIDE.md#52-point-扩展方法)
- **快速使用**: [QUICK_REFERENCE.md § 1. Point 操作](QUICK_REFERENCE.md#1-point-操作)
- **API 文档**: `SharedRust/src/map.rs`

### 枚举类型
- **C# → Rust 映射**: [PORTING_GUIDE.md § 1. 枚举类型](PORTING_GUIDE.md#1-枚举类型-enums)
- **快速使用**: [README.md § 基础使用](README.md#基础使用)
- **API 文档**: `SharedRust/src/enums.rs`

### 数据结构
- **C# → Rust 映射**: [PORTING_GUIDE.md § 4. 数据结构](PORTING_GUIDE.md#4-数据结构-data)
- **API 文档**: `SharedRust/src/data/`

---

## 💡 常见任务

### 任务 1: 在 ClientRust 中发送登录数据包

1. 查看快速参考: [QUICK_REFERENCE.md § 角色登录流程](QUICK_REFERENCE.md#角色登录流程)
2. 查看详细映射: [PORTING_GUIDE.md § 客户端数据包](PORTING_GUIDE.md#32-客户端数据包-clientpackets)

### 任务 2: 计算两点之间的方向

1. 查看快速参考: [QUICK_REFERENCE.md § 方向计算](QUICK_REFERENCE.md#2-方向计算)
2. 查看函数说明: [PORTING_GUIDE.md § 方向和几何函数](PORTING_GUIDE.md#51-方向和几何函数)

### 任务 3: 查找某个 C# 类的 Rust 版本

1. 查看总览表: [PORTING_GUIDE.md § 模块对应关系总览](PORTING_GUIDE.md#-模块对应关系总览)
2. 查看详细映射: [PORTING_GUIDE.md § 详细模块映射](PORTING_GUIDE.md#-详细模块映射)

### 任务 4: 实现新的数据包

1. 查看已实现示例: `SharedRust/src/packets/client/`
2. 参考基础设施: [PORTING_GUIDE.md § 数据包基础设施](PORTING_GUIDE.md#31-数据包基础设施)
3. 查看移植质量标准: [MIGRATION_REPORT.md § 移植质量保证](MIGRATION_REPORT.md#-移植质量保证)

---

## 🚀 快速跳转

### ClientRust 开发者
1. [添加依赖](README.md#添加依赖)
2. [基础导入](QUICK_REFERENCE.md#基础导入)
3. [常用代码片段](QUICK_REFERENCE.md#-常用代码片段)
4. [已实现的数据包列表](QUICK_REFERENCE.md#-已实现的客户端数据包)

### 从 C# 迁移的开发者
1. [C# → Rust 命名转换](PORTING_GUIDE.md#命名转换)
2. [类型对照表](PORTING_GUIDE.md#类型对照)
3. [模块映射表](PORTING_GUIDE.md#-模块对应关系总览)
4. [常见问题 FAQ](PORTING_GUIDE.md#-常见问题-faq)

### 贡献者
1. [项目状态](README.md#-项目状态)
2. [下一步计划](README.md#-下一步计划)
3. [移植进度](MIGRATION_REPORT.md#-总结)
4. [开发指南](README.md#-开发)

---

## 📞 需要帮助?

### 找不到对应的 Rust 模块?
→ 查看 [PORTING_GUIDE.md § 模块对应关系总览](PORTING_GUIDE.md#-模块对应关系总览)

### 不知道如何使用某个功能?
→ 查看 [QUICK_REFERENCE.md](QUICK_REFERENCE.md)

### 想了解移植进度?
→ 查看 [README.md § 项目状态](README.md#-项目状态)

### 需要实现新功能?
→ 参考已实现的代码和 [MIGRATION_REPORT.md](MIGRATION_REPORT.md)

---

**祝您使用愉快! 🎉**
