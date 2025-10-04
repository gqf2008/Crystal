# P0-3 资源加载系统实现报告

**完成时间:** 2025-10-04  
**任务:** P0-3 实现资源加载系统 (texture_loader.rs)

## 📋 概述

成功实现了MIR2 .lib图像库文件的加载系统,可以解析和加载游戏纹理资源。

---

## 🏗️ 架构设计

### 1. 模块结构

```
ClientRust/src/graphics/
├── mod.rs
└── texture_loader.rs  (新增 - 400行)
```

### 2. 核心组件

#### **MLibrary** - .lib文件解析器
```rust
pub struct MLibrary {
    path: PathBuf,
    header: LibraryHeader,
    indices: Vec<ImageIndex>,
    cached_info: HashMap<usize, ImageInfo>,
}
```

**功能:**
- 打开.lib文件并解析文件头
- 读取图像索引表
- 按需加载单个图像
- 缓存图像元数据

**方法:**
- `open(path)` - 打开库文件
- `count()` - 获取图像数量
- `get_image_info(index)` - 读取图像信息(不解压)
- `load_image_data(index)` - 加载并解压图像数据
- `load_color_image(index)` - 加载为egui ColorImage

#### **TextureManager** - 纹理管理器
```rust
pub struct TextureManager {
    libraries: HashMap<String, MLibrary>,
    textures: HashMap<TextureKey, TextureHandle>,
}
```

**功能:**
- 管理多个图像库
- 缓存已加载的纹理(避免重复加载)
- 与egui渲染系统集成

**方法:**
- `new()` - 创建管理器
- `load_library(name, path)` - 加载图像库
- `get_texture(ctx, library, index)` - 获取或加载纹理
- `get_image_info(library, index)` - 仅获取图像信息
- `clear_cache()` - 清除缓存

---

## 📦 .lib文件格式解析

### 文件结构

```
+-------------------+
| Header (12 bytes) |
|-------------------|
| Version    (i32)  |  文件格式版本
| Count      (i32)  |  图像数量
| FrameSeek  (i32)  |  动画帧偏移
+-------------------+
| Index Table       |
|-------------------|
| Offset[0]  (i32)  |  图像0的文件偏移
| Offset[1]  (i32)  |  图像1的文件偏移
| ...               |
| Offset[n]  (i32)  |  图像n的文件偏移
+-------------------+
| Image 0 Data      |
|-------------------|
| Width      (i16)  |
| Height     (i16)  |
| X          (i16)  |  渲染偏移X
| Y          (i16)  |  渲染偏移Y
| ShadowX    (i16)  |
| ShadowY    (i16)  |
| Shadow     (u8)   |  阴影标志(bit7=HasMask)
| Length     (i32)  |  压缩数据长度
| [GZip Data ...]   |  BGRA格式压缩像素
+-------------------+
| Image 1 Data      |
+-------------------+
| ...               |
+-------------------+
```

### 图像数据格式

1. **压缩格式:** GZip (使用flate2库解压)
2. **像素格式:** BGRA8 (Blue-Green-Red-Alpha, 每像素4字节)
3. **布局:** 行优先 (top-to-bottom, left-to-right)
4. **需要转换:** BGRA → RGBA (egui使用RGBA格式)

### 第二层(Mask Layer)

如果 `Shadow & 0x80 != 0`, 图像有第二层:

```
+-------------------+
| Mask Layer        |
|-------------------|
| MaskWidth  (i16)  |
| MaskHeight (i16)  |
| MaskX      (i16)  |
| MaskY      (i16)  |
| MaskLength (i32)  |
| [GZip Data ...]   |
+-------------------+
```

**当前实现:** 仅读取主层,第二层支持待实现

---

## 🔧 技术实现细节

### 1. 字节序

MIR2使用 **小端序** (Little-Endian):
```rust
fn read_i32<R: Read>(reader: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}
```

### 2. GZip解压

```rust
use flate2::read::GzDecoder;

let mut decompressor = GzDecoder::new(&compressed[..]);
let mut decompressed = Vec::new();
decompressor.read_to_end(&mut decompressed)?;
```

### 3. BGRA → RGBA转换

```rust
for chunk in data.chunks_exact(4) {
    let b = chunk[0];
    let g = chunk[1];
    let r = chunk[2];
    let a = chunk[3];
    rgba_data.push(r);  // 交换B和R
    rgba_data.push(g);
    rgba_data.push(b);
    rgba_data.push(a);
}
```

### 4. egui纹理加载

```rust
let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba_data);
let handle = ctx.load_texture(texture_name, color_image, Default::default());
```

---

## 🎯 集成到应用

### 在 `MirClientApp` 中添加

```rust
pub struct MirClientApp {
    // ...
    texture_manager: TextureManager,
    // ...
}

impl MirClientApp {
    pub fn new(cc: &eframe::CreationContext, ...) -> Self {
        let mut texture_manager = TextureManager::new();
        
        // 加载基础UI库
        let _ = texture_manager.load_library(
            "Prguse", 
            std::path::Path::new("Data/Prguse.lib")
        );
        
        Self {
            texture_manager,
            // ...
        }
    }
}
```

### 使用示例

```rust
// 在场景渲染时获取纹理
if let Ok((info, texture)) = self.texture_manager.get_texture(
    ui.ctx(),
    "Prguse",  // 库名
    123,       // 图像索引
) {
    // 计算渲染位置(考虑偏移)
    let x = base_x + info.x as f32;
    let y = base_y + info.y as f32;
    
    // 渲染
    ui.image(texture.id(), [info.width as f32, info.height as f32]);
}
```

---

## 📊 性能优化

### 1. 懒加载 (Lazy Loading)

- 只加载需要的图像,不是整个库
- 使用索引表快速定位
- 避免启动时长时间加载

### 2. 缓存策略

```rust
cached_info: HashMap<usize, ImageInfo>    // 元数据缓存
textures: HashMap<TextureKey, TextureHandle>  // GPU纹理缓存
```

- **元数据缓存:** 避免重复解析图像头
- **纹理缓存:** 避免重复解压和上传GPU

### 3. 内存管理

```rust
pub fn clear_cache(&mut self) {
    self.textures.clear();  // 清除所有纹理
}
```

场景切换时可清除不需要的纹理

---

## 🧪 测试计划

### 单元测试 (TODO)

```rust
#[test]
fn test_library_open() {
    let lib = MLibrary::open("Data/Prguse.lib").unwrap();
    assert!(lib.count() > 0);
}

#[test]
fn test_load_image() {
    let mut lib = MLibrary::open("Data/Prguse.lib").unwrap();
    let (info, data) = lib.load_image_data(0).unwrap();
    assert_eq!(data.len(), (info.width * info.height * 4) as usize);
}
```

### 集成测试

1. **启动测试:** 程序启动时加载Prguse.lib
2. **渲染测试:** LoginScene显示背景图像
3. **性能测试:** 监控加载时间和内存使用

---

## 📦 依赖项

### Cargo.toml

```toml
[dependencies]
flate2 = "1"  # GZip解压
egui = "0.29"  # UI框架
```

---

## 🎨 使用的库文件

### 优先级列表

| 库名 | 路径 | 用途 | 优先级 |
|------|------|------|--------|
| Prguse | Data/Prguse.lib | 主UI元素 | P0 |
| Prguse2 | Data/Prguse2.lib | 扩展UI | P1 |
| ChrSel | Data/ChrSel.lib | 角色选择界面 | P1 |
| Title | Data/Title.lib | 标题/Logo | P1 |
| Background | Data/Background.lib | 背景图 | P2 |
| Items | Data/Items.lib | 物品图标 | P2 |

### 加载顺序建议

1. **启动时:** Prguse (LoginScene需要)
2. **登录成功:** ChrSel, Prguse2
3. **进入游戏:** Items, Monsters, Map等

---

## ✅ 完成状态

### 已实现
- ✅ .lib文件格式解析
- ✅ 文件头和索引表读取
- ✅ GZip解压
- ✅ BGRA→RGBA转换
- ✅ egui纹理集成
- ✅ TextureManager缓存系统
- ✅ 集成到MirClientApp

### 待实现
- ⏳ Mask Layer(第二层)支持
- ⏳ 动画帧读取(FrameSet)
- ⏳ 错误处理优化
- ⏳ 单元测试
- ⏳ 性能监控

### 已知限制
1. **第二层未实现:** 某些特效图像可能显示不完整
2. **无动画支持:** 暂时只能加载静态图像
3. **路径硬编码:** Data/路径应该从settings读取
4. **无异步加载:** 大量纹理加载可能卡顿

---

## 📝 下一步计划

### P0-4: 实现音频系统
- sound_loader.rs - 加载.wav音效文件
- 背景音乐播放
- 音效管理器

### P1: 完善资源系统
- 实现动画帧支持
- 添加Mask Layer渲染
- 异步资源加载
- 资源预加载策略

### P1: LoginScene纹理渲染
- 加载并显示背景图
- UI按钮使用纹理
- Logo/Title显示

---

## 🔗 参考文件

### C#源码
- `Client/MirGraphics/MLibrary.cs` (1087行)
- `LibraryEditor/Graphics/MLibraryV2.cs`
- `LibraryViewer/MLibrary.cs`

### Rust实现
- `ClientRust/src/graphics/texture_loader.rs` (400行)
- `ClientRust/src/graphics/mod.rs`
- `ClientRust/src/app.rs`

---

## 🎉 成果

**代码量:** ~400行Rust代码  
**编译状态:** ✅ 通过 (仅有ambiguous re-exports警告)  
**测试状态:** ⏳ 待验证(需要实际.lib文件)  
**文档状态:** ✅ 完整

下一步将实现音频加载系统(sound_loader.rs),完成P0阶段所有基础功能!
