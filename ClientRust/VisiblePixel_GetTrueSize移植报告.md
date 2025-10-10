# VisiblePixel 和 GetTrueSize 方法移植完成报告

## 📋 移植概述

将 C# `MLibrary.cs` 中的 `MImage.VisiblePixel` 和 `MImage.GetTrueSize` 方法移植到 Rust `mlibrary.rs` 模块的 `ImageInfo` 结构体中。

## 📅 日期
2025年10月10日

## 🎯 移植的方法

### 1. `ImageInfo::visible_pixel()`
- **C# 原型**: `public unsafe bool VisiblePixel(Point p)`
- **Rust 原型**: `pub fn visible_pixel(&self, x: i32, y: i32, rgba_data: &[u8]) -> bool`
- **功能**: 检查指定像素是否可见（alpha通道 > 0）

### 2. `ImageInfo::get_true_size()`
- **C# 原型**: `public Size GetTrueSize()`
- **Rust 原型**: `pub fn get_true_size(&self, rgba_data: &[u8]) -> (i16, i16)`
- **功能**: 获取图像实际显示尺寸（去除透明边缘）

## 📝 C# 原始实现

### VisiblePixel
```csharp
// Client/MirGraphics/MLibrary.cs Line 1019-1037
public unsafe bool VisiblePixel(Point p)
{
    if (p.X < 0 || p.Y < 0 || p.X >= Width || p.Y >= Height)
        return false;

    int w = Width;

    bool result = false;
    if (Data != null)
    {
        int x = p.X;
        int y = p.Y;
        
        int index = (y * (w << 2)) + (x << 2) + 3;
        
        byte col = Data[index];

        if (col == 0) return false;
        else return true;
    }
    return result;
}
```

### GetTrueSize
```csharp
// Client/MirGraphics/MLibrary.cs Line 1039-1121
public Size GetTrueSize()
{
    if (TrueSize != Size.Empty) return TrueSize;

    int l = 0, t = 0, r = Width, b = Height;

    bool visible = false;
    // 1. 从左到右扫描
    for (int x = 0; x < r; x++)
    {
        for (int y = 0; y < b; y++)
        {
            if (!VisiblePixel(new Point(x, y))) continue;
            visible = true;
            break;
        }
        if (!visible) continue;
        l = x;
        break;
    }

    // 2. 从上到下扫描
    visible = false;
    for (int y = 0; y < b; y++)
    {
        for (int x = l; x < r; x++)
        {
            if (!VisiblePixel(new Point(x, y))) continue;
            visible = true;
            break;
        }
        if (!visible) continue;
        t = y;
        break;
    }

    // 3. 从右到左扫描
    visible = false;
    for (int x = r - 1; x >= l; x--)
    {
        for (int y = 0; y < b; y++)
        {
            if (!VisiblePixel(new Point(x, y))) continue;
            visible = true;
            break;
        }
        if (!visible) continue;
        r = x + 1;
        break;
    }

    // 4. 从下到上扫描
    visible = false;
    for (int y = b - 1; y >= t; y--)
    {
        for (int x = l; x < r; x++)
        {
            if (!VisiblePixel(new Point(x, y))) continue;
            visible = true;
            break;
        }
        if (!visible) continue;
        b = y + 1;
        break;
    }

    TrueSize = Rectangle.FromLTRB(l, t, r, b).Size;
    return TrueSize;
}
```

## ✅ Rust 实现

### 1. visible_pixel
```rust
// ClientRust/src/graphics/mlibrary.rs
pub fn visible_pixel(&self, x: i32, y: i32, rgba_data: &[u8]) -> bool {
    // 边界检查
    if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
        return false;
    }

    let w = self.width as usize;
    
    // 计算索引: (y * width * 4) + (x * 4) + 3
    // w << 2 等价于 w * 4
    let index = ((y as usize) * (w << 2)) + ((x as usize) << 2) + 3;

    // 检查 alpha 通道
    if index < rgba_data.len() {
        rgba_data[index] != 0
    } else {
        false
    }
}
```

### 2. get_true_size
```rust
pub fn get_true_size(&self, rgba_data: &[u8]) -> (i16, i16) {
    let mut l = 0i32;
    let mut t = 0i32;
    let mut r = self.width as i32;
    let mut b = self.height as i32;

    // 1. 从左到右扫描，找到第一列包含可见像素
    let mut visible = false;
    for x in 0..r {
        for y in 0..b {
            if !self.visible_pixel(x, y, rgba_data) {
                continue;
            }
            visible = true;
            break;
        }
        if !visible {
            continue;
        }
        l = x;
        break;
    }

    // 2. 从上到下扫描
    visible = false;
    for y in 0..b {
        for x in l..r {
            if !self.visible_pixel(x, y, rgba_data) {
                continue;
            }
            visible = true;
            break;
        }
        if !visible {
            continue;
        }
        t = y;
        break;
    }

    // 3. 从右到左扫描
    visible = false;
    for x in (l..r).rev() {
        for y in 0..b {
            if !self.visible_pixel(x, y, rgba_data) {
                continue;
            }
            visible = true;
            break;
        }
        if !visible {
            continue;
        }
        r = x + 1;
        break;
    }

    // 4. 从下到上扫描
    visible = false;
    for y in (t..b).rev() {
        for x in l..r {
            if !self.visible_pixel(x, y, rgba_data) {
                continue;
            }
            visible = true;
            break;
        }
        if !visible {
            continue;
        }
        b = y + 1;
        break;
    }

    // 返回宽度和高度
    let width = (r - l) as i16;
    let height = (b - t) as i16;
    
    (width, height)
}
```

## 🔄 C# 与 Rust 对应关系

| C# 实现 | Rust 实现 | 说明 |
|--------|----------|------|
| `Point p` | `x: i32, y: i32` | 分离为两个参数 |
| `Data` 指针 | `rgba_data: &[u8]` | 安全的切片引用 |
| `(y * (w << 2)) + (x << 2) + 3` | 相同 | 索引计算一致 |
| `Size` | `(i16, i16)` | 返回元组 |
| `TrueSize` 缓存 | 无缓存 | 由调用方缓存 |
| `Rectangle.FromLTRB()` | 直接计算 | 更简洁 |

## 🎨 关键设计差异

### 1. 内存安全
**C#**: 使用 `unsafe` 指针直接访问纹理数据
```csharp
public unsafe byte* Data;
int index = (y * (w << 2)) + (x << 2) + 3;
byte col = Data[index];
```

**Rust**: 使用安全的切片引用
```rust
fn visible_pixel(&self, x: i32, y: i32, rgba_data: &[u8]) -> bool {
    let index = ((y as usize) * (w << 2)) + ((x as usize) << 2) + 3;
    if index < rgba_data.len() {
        rgba_data[index] != 0
    } else {
        false
    }
}
```

### 2. 参数设计
**C#**: 依赖内部状态（`Data` 字段）
```csharp
public unsafe bool VisiblePixel(Point p)
{
    if (Data != null) {
        // 使用 this.Data
    }
}
```

**Rust**: 显式传入数据（函数式）
```rust
pub fn visible_pixel(&self, x: i32, y: i32, rgba_data: &[u8]) -> bool {
    // rgba_data 作为参数传入
}
```

### 3. 缓存策略
**C#**: 内部缓存 `TrueSize`
```csharp
public Size TrueSize;
public Size GetTrueSize()
{
    if (TrueSize != Size.Empty) return TrueSize;
    // ... 计算 ...
    TrueSize = Rectangle.FromLTRB(l, t, r, b).Size;
    return TrueSize;
}
```

**Rust**: 无内部缓存（由调用方决定）
```rust
pub fn get_true_size(&self, rgba_data: &[u8]) -> (i16, i16) {
    // 每次重新计算
    // 调用方可以缓存结果
}
```

## 💡 使用示例

### visible_pixel
```rust
// 加载图像数据
let mut info = lib.get_image_info(0)?;
let mut file = File::open("Data/Items.Lib")?;
file.seek(SeekFrom::Start(offset + 17))?;

let (main_rgba, _) = info.create_texture(&mut ctx, &mut file)?;

// 检查像素可见性
let is_visible = info.visible_pixel(10, 20, &main_rgba);
println!("像素 (10, 20) 可见: {}", is_visible);
```

### get_true_size
```rust
// 获取实际显示尺寸
let (true_width, true_height) = info.get_true_size(&main_rgba);
println!("实际尺寸: {}x{}", true_width, true_height);

// 可以缓存结果
let true_size = info.get_true_size(&main_rgba);
// 后续使用 true_size
```

## 🔧 算法复杂度

### visible_pixel
- **时间复杂度**: O(1)
- **空间复杂度**: O(1)

### get_true_size
- **最坏情况**: O(width × height) - 图像全透明或只有一个像素
- **最佳情况**: O(width + height) - 边缘就有可见像素
- **平均情况**: O(width × height / 2)

## ⚠️ 注意事项

1. **rgba_data 参数**: 必须是 RGBA 格式，每个像素4字节
2. **坐标系统**: 左上角为原点 (0, 0)
3. **边界检查**: 自动处理越界情况，返回 false
4. **性能**: `get_true_size` 是 CPU 密集型操作，建议缓存结果

## 📊 移植统计

| 项目 | 数值 |
|------|------|
| 移植方法数 | 2 个 |
| 代码行数 | ~170 行（含注释） |
| C# 代码行数 | ~103 行 |
| 编译状态 | ⚠️ 部分通过（有未解决的依赖问题） |
| 文档完整度 | 100% |

## 🚀 下一步工作

### 需要解决的问题
1. ⚠️ `load_rgba_data` 方法被注释，需要修复调用处
2. ⚠️ 其他文件中的编译错误需要修复

### 后续移植
1. `DisposeTexture` - 已完成 ✅
2. 其他 MLibrary 绘制方法

## ✅ 总结

成功移植了 `VisiblePixel` 和 `GetTrueSize` 两个方法到 Rust，主要改进：

✅ **内存安全**: 使用切片而非 unsafe 指针  
✅ **类型安全**: 显式参数，无隐式状态依赖  
✅ **函数式**: 无副作用，易于测试  
✅ **文档完整**: 详细的注释和 C# 对照  
✅ **算法一致**: 完全保持原有逻辑

这两个方法主要用于：
- **VisiblePixel**: 鼠标悬停检测、碰撞检测
- **GetTrueSize**: UI 布局、精确显示区域计算
