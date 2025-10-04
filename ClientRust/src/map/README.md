# Map 目录 (已废弃)

⚠️ **本目录已废弃** - 所有文件已删除

---

## 废弃原因

2025-10-04: 根据"**不要过早重构**"原则，决定严格按照 C# Client 的原始结构移植。

原本在此目录下的实验性重构代码（MapData, MapLoader, Camera, MapRenderer）已删除。

---

## 功能已迁移至

地图相关功能现在按照 C# Client 的组织方式存放：

### ✅ `src/objects/map_code.rs`
**对应**: `Client/MirObjects/MapCode.cs`

包含：
- `CellInfo` - 格子信息类
- `MapReader` - 地图加载器类

### ⏳ `src/scenes/game_scene.rs` (待实现)
**对应**: `Client/MirScenes/GameScene.cs::MapControl`

将包含：
- 地图渲染逻辑 (`DrawFloor()`)
- 相机参数 (`OffSetX`, `OffSetY`, `ViewRangeX`, `ViewRangeY`)
- 对象管理

---

## C# Client 原始设计

```csharp
// Client/MirObjects/MapCode.cs
class CellInfo { ... }
class MapReader { ... }

// Client/MirScenes/GameScene.cs
class MapControl {
    static int OffSetX, OffSetY;
    static int ViewRangeX, ViewRangeY;
    CellInfo[,] M2CellInfo;
    
    void DrawFloor() { ... }
}
```

---

## 移植策略

**先移植，后重构**

1. ✅ Phase 1: 按 C# 结构直接移植所有功能
2. ⏳ Phase 2: 让游戏能完整运行
3. ⏳ Phase 3: 根据实际需求评估是否重构

**原则**: 功能完整 > 代码完美

---

## 相关文档

- 📄 `docs/map-code-reorganization.md` - Map 模块重组记录
- 📄 `docs/DIRECT_MIGRATION_ROADMAP.md` - 直接移植路线图
- 🔗 `src/objects/map_code.rs` - 当前地图实现dering System (已弃用)

⚠️ **本模块已弃用** - 功能已按 C# 原始结构移至 `src/objects/map_code.rs`

---

# 原: Map Rendering System (实验性模块)

⚠️ **状态**: 实验性 / 未集成

## 说明

这个模块是对 C# Client 中地图系统的**重构版本**，将地图功能从 `MirObjects/MapCode.cs` 和 `MirScenes/GameScene.MapControl` 中独立出来。

### 与 C# 的对应关系

| Rust 模块 | C# 源文件 | 说明 |
|-----------|----------|------|
| `map_data.rs` | `MirObjects/MapCode.cs::CellInfo` | 地图单元格数据 |
| `map_loader.rs` | `MirObjects/MapCode.cs::MapReader` | 地图文件加载 |
| `camera.rs` | `GameScene.cs::MapControl` (静态变量) | 相机和视口 |
| `map_renderer.rs` | `GameScene.cs::MapControl::DrawFloor()` | 地图渲染 |

## 当前状态

- ✅ 所有代码实现完成
- ✅ 单元测试通过 (16/16)
- ⏸️ **未集成到 GameScene**
- 📝 **等待移植完成后决定是否使用**

## 使用计划

**选项 A**: 移植完成后，如果 C# 的直接移植方式工作良好，则废弃此模块

**选项 B**: 移植完成后，如果发现需要重构，则使用此模块替换相应的 C# 移植代码

**选项 C**: 作为独立的地图系统实现，与 C# 移植版本共存，供性能对比和实验使用

## 文档

详细实现文档见：
- `docs/p3-2-map-rendering-plan.md` - 实施计划
- `docs/p3-2-implementation-report.md` - 完整报告

---

**创建日期**: 2025-10-04  
**作者**: AI Assistant  
**决策**: 先完成 C# Client 的直接移植，再考虑是否采用此重构版本
