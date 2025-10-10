# 地图查看器 - 快速参考

## 🚀 快速启动

```powershell
# 加载默认地图
cargo run --example map_viewer --release

# 加载指定地图
cargo run --example map_viewer --release Map/r001.map
```

## 🎮 操作指南

| 操作 | 功能 |
|------|------|
| **按住左键拖拽** | 移动视角 |
| **滚轮向上** | 放大 (Zoom In) |
| **滚轮向下** | 缩小 (Zoom Out) |
| **ESC** | 退出程序 |

## 📊 UI 信息说明

左上角显示：
```
Map: Map/r001.map          ← 当前地图文件
Size: 200x200              ← 地图尺寸（格子）
FPS: 60                    ← 当前帧率
Zoom: 1.50x                ← 缩放级别
Camera: (4800, 3200)       ← 相机世界坐标
Grid: (100, 100)           ← 鼠标悬停格子坐标

按住左键拖拽 | 滚轮缩放
```

## 🗺️ 常用地图

```powershell
# 银杏山谷 (200x200)
cargo run --example map_viewer --release Map/r001.map

# 白日门 (200x200)
cargo run --example map_viewer --release Map/r002.map

# 比奇城
cargo run --example map_viewer --release Map/3.map

# 盟重土城
cargo run --example map_viewer --release Map/1.map
```

## 💡 使用技巧

### 1. 快速定位
- 程序启动时相机自动定位到地图中心
- 拖拽到边缘后，缩小查看全貌
- 再放大查看感兴趣的区域

### 2. 查看细节
- 滚轮放大到 2x-4x 查看瓦片细节
- 移动鼠标查看格子坐标
- 观察 Middle/Front 层的动画

### 3. 性能监控
- 观察左上角 FPS 是否稳定在 60
- 如果 FPS 低，检查是否在 Release 模式运行
- 缩小视图可以减少渲染瓦片数量

## 🔧 故障排除

### 问题 1: 地图加载失败
```
错误: Map Type 100 not yet implemented
解决: 选择其他地图，如 Map/r001.map
```

### 问题 2: FPS 很低
```
原因: 可能在 Debug 模式运行
解决: 确保使用 --release 参数
```

### 问题 3: 窗口太小/太大
```
默认: 1280x960
调整: 修改 examples/map_viewer.rs 第 467 行
      .dimensions(1920.0, 1080.0)  // 改为你的分辨率
```

### 问题 4: 地图显示不全
```
解决: 缩小视图（滚轮向下），然后拖拽查看其他区域
```

## 📐 坐标系统说明

### 地图格子
- 每格大小: 48x32 像素
- 坐标原点: 左上角 (0, 0)
- Back 层: 只渲染偶数坐标 (x%2==0 && y%2==0)

### 相机坐标
- 单位: 像素
- 初始位置: 地图中心
- 范围: 可以超出地图边界

### 缩放级别
- 最小: 0.25x (俯瞰全图)
- 正常: 1.0x (原始分辨率)
- 最大: 4.0x (细节放大)

## ⚡ 性能优化

### 已实现的优化
- ✅ 纹理缓存 (第一帧加载，后续复用)
- ✅ 视锥剔除 (只渲染可见区域)
- ✅ Release 模式优化
- ✅ 智能清理 (移除长期未使用的纹理)

### 性能指标
- 200x200 地图: ~60 FPS
- 可见瓦片数: ~800 (1280x960 窗口)
- 纹理缓存命中率: >99%

## 🎯 快速测试流程

```powershell
# 1. 编译并运行
cd ClientRust
cargo run --example map_viewer --release Map/r001.map

# 2. 测试拖拽
#    按住左键，移动鼠标，观察视角移动

# 3. 测试缩放
#    滚动鼠标滚轮，观察缩放效果

# 4. 查看信息
#    移动鼠标到不同位置，观察左上角坐标变化

# 5. 检查 FPS
#    确认左上角显示 FPS: 60
```

## 📝 代码位置

```
examples/map_viewer.rs          ← 主程序
MAP_VIEWER_使用说明.md          ← 详细文档
地图查看器开发总结.md            ← 技术总结
```

## 🆘 需要帮助？

查看完整文档:
```powershell
# 用户手册
code MAP_VIEWER_使用说明.md

# 技术总结
code 地图查看器开发总结.md
```

---

**版本**: v1.0  
**更新**: 2025-10-09  
**状态**: ✅ 可用
