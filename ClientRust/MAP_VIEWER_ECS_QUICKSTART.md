# 🚀 ECS 地图查看器 - 快速开始

## 立即运行

```powershell
# 开发模式 (快速编译)
cargo run --bin map_viewer_ecs

# 发布模式 (最佳性能)
cargo run --bin map_viewer_ecs --release
```

## 快捷键速查

```
🗺️  地图操作
  [M] - 选择地图文件

🎨 图层控制
  [1] - Back 层 (背景层)
  [2] - Middle 层 (中间层)  
  [3] - Front 层 (前景层)

🔍 调试工具
  [G] - 网格线
  [O] - 障碍物高亮
  [A] - 动画播放/暂停
  [B] - 纹理边框

🖱️  视角控制
  拖拽 - 移动视角
  滚轮 - 缩放视图
  
⌨️  系统
  [ESC] - 退出程序
```

## 第一次运行

1. **确保数据文件存在**:
   ```
   Map/0.map          ← 默认地图
   Data/              ← 地图库目录
   ```

2. **运行程序**:
   ```powershell
   cargo run --bin map_viewer_ecs --release
   ```

3. **成功标志**:
   ```
   📚 正在初始化地图库...
   ✅ 地图库初始化完成
   🗺️ 正在加载地图: Map/0.map
   ✅ 地图加载完成: 200x200
   📦 正在加载地图瓦片到 ECS...
   ✅ 加载完成: 12543 个瓦片实体

   🎮 ECS 地图查看器已启动!
   ```

## 常见问题

### Q: 找不到地图文件
**A**: 按 `M` 键手动选择 Map 目录下的 .map 文件

### Q: 画面卡顿
**A**: 使用 `--release` 模式编译

### Q: 看不到任何东西
**A**: 尝试按 `1` / `2` / `3` 键确保图层已开启

### Q: 想看原版对比
**A**: 运行 OOP 版本:
```powershell
cargo run --bin map_viewer --release
```

## 性能对比

| 指标 | OOP 版 | ECS 版 |
|------|--------|--------|
| 启动时间 | ~1s | ~1s |
| 内存占用 | 适中 | 更优 |
| 扩展性 | 中 | 高 |
| 代码量 | 1598行 | 1103行 |

## 下一步

查看完整文档: `ECS_MAP_VIEWER_SUCCESS.md`

开始集成到 GameScene: `ECS_ARCHITECTURE.md`

---
**享受你的 ECS 之旅！** 🎉
