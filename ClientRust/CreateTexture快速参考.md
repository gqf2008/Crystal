# CreateTexture 快速参考

## 🎯 用途
从 .lib 文件中读取并解压图像数据，转换为 RGBA 格式。

## 📝 函数签名

```rust
impl ImageInfo {
    pub fn create_texture<R: std::io::Read>(
        &self,
        reader: &mut R,
    ) -> Result<(Vec<u8>, Option<Vec<u8>>), std::io::Error>
}
```

## 📥 输入
- `self`: ImageInfo 实例（包含宽度、高度、压缩长度等元数据）
- `reader`: 已定位到压缩数据起始位置的读取器

## 📤 输出
- `Ok((main_image, mask_image))`: 
  - `main_image`: Vec<u8> - RGBA格式主图像（宽×高×4字节）
  - `mask_image`: Option<Vec<u8>> - 可选遮罩层
- `Err(io::Error)`: 读取或解压失败

## 💡 使用示例

### 基础用法
```rust
// 1. 打开图像库并获取信息
let mut lib = MLibrary::open("Data/Items.Lib")?;
let info = lib.get_image_info(0)?;

// 2. 定位到压缩数据
let mut file = File::open("Data/Items.Lib")?;
file.seek(SeekFrom::Start(offset + 17))?; // +17跳过ImageInfo头

// 3. 创建纹理
let (rgba_data, mask) = info.create_texture(&mut file)?;

// 4. 使用数据
println!("图像尺寸: {}x{}", info.width, info.height);
println!("数据大小: {} 字节", rgba_data.len());
if let Some(mask_data) = mask {
    println!("遮罩数据大小: {} 字节", mask_data.len());
}
```

### 创建 ggez 纹理
```rust
use ggez::graphics::{Image, ImageFormat};

let (rgba_data, _) = info.create_texture(&mut file)?;

let texture = Image::from_pixels(
    ctx,
    &rgba_data,
    ImageFormat::Rgba8Unorm,
    info.width as u32,
    info.height as u32,
);
```

## ⚙️ 内部流程

```
1. 读取主图像压缩数据 (self.length 字节)
   ↓
2. GZip 解压
   ↓
3. BGRA → RGBA 转换 + 黑色透明化
   ↓
4. 如果 has_mask:
   ├─ 跳过 12 字节遮罩头
   ├─ 读取遮罩压缩数据 (self.mask_length 字节)
   ├─ GZip 解压
   └─ BGRA → RGBA 转换
   ↓
5. 返回 (主图像, 遮罩)
```

## 🎨 数据格式

### 输入格式（.lib 文件）
```
[ImageInfo 17字节]
[GZip压缩的BGRA数据]
[如果has_mask:]
  [遮罩头 12字节]
  [GZip压缩的BGRA数据]
```

### 输出格式
```rust
// RGBA, 4字节/像素
[R, G, B, A, R, G, B, A, ...]
// 总大小: width * height * 4
```

## 🔧 关键特性

### 1. 透明色处理
```rust
if r == 0 && g == 0 && b == 0 {
    a = 0;  // 黑色 → 透明
}
```

### 2. 颜色通道转换
```rust
// 输入: B G R A
// 输出: R G B A
rgba_data.push(r);  // R
rgba_data.push(g);  // G
rgba_data.push(b);  // B
rgba_data.push(a);  // A
```

### 3. 数据大小验证
```rust
let expected = width * height * 4;
if decompressed.len() != expected {
    // 过长: 截断
    // 过短: 填充0
}
```

## ⚠️ 注意事项

1. **Reader 位置**: 调用前必须定位到压缩数据开始处（ImageInfo头之后）
2. **内存分配**: 会分配 width×height×4 字节内存
3. **遮罩头**: 12字节遮罩头会被自动跳过（信息已在ImageInfo中）
4. **错误处理**: 使用 `?` 操作符传播 IO 错误

## 🔄 与 C# 对比

| 特性 | C# | Rust |
|-----|-----|------|
| 颜色格式 | A8R8G8B8 (BGRA) | RGBA |
| 内存管理 | 手动 Lock/Unlock | 自动（Vec） |
| 错误处理 | 异常 | Result<T, E> |
| 遮罩返回 | MaskImage 字段 | Option<Vec<u8>> |
| 透明处理 | 隐式 | 显式 |

## 📚 相关方法

- `ImageInfo::from_reader()` - 读取图像元数据
- `ImageInfo::decompress_image()` - 解压单个图层（私有）
- `MLibrary::load_rgba_data()` - 高层封装，包含缓存

## 🧪 测试建议

```rust
#[test]
fn test_create_texture() {
    let info = ImageInfo {
        width: 100,
        height: 100,
        length: 1234,
        has_mask: false,
        ..Default::default()
    };
    
    let mut reader = create_test_reader(); // 准备测试数据
    let (data, mask) = info.create_texture(&mut reader).unwrap();
    
    assert_eq!(data.len(), 100 * 100 * 4);
    assert!(mask.is_none());
}
```

## 🚀 性能提示

- 对于频繁访问的图像，使用 `MLibrary::get_or_create_texture()` 缓存
- 大批量加载时考虑并行解压（需额外实现）
- 解压是 CPU 密集型操作，避免在渲染循环中调用
