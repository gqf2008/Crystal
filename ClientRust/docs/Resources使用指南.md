# Resources 模块快速参考

## 快速开始

```rust
use mir2_client::resources::Images;
```

## 基础用法

### 获取原始字节
```rust
let bytes: &'static [u8] = Images::blue_progress();
```

### 加载为图像（推荐）
```rust
let img = Images::load_blue_progress()?;  // 返回 DynamicImage
let (width, height) = img.dimensions();
```

## 所有可用资源

### 进度条 (Progress Bars)
```rust
Images::blue_progress() / load_blue_progress()         // 550x13
Images::green_progress() / load_green_progress()       // 550x13
Images::new_progress_end_blue()                        // 端盖
Images::new_progress_end_green()                       // 端盖
```

### 启动按钮 (Launch Button)
```rust
Images::launch_base() / load_launch_base()             // 115x53
Images::launch_hover() / load_launch_hover()
Images::launch_pressed() / load_launch_pressed()
Images::launch_base1()                                 // 变体
```

### 配置按钮 (Config Button)
```rust
Images::config_base() / load_config_base()             // 19x19
Images::config_hover() / load_config_hover()
Images::config_pressed() / load_config_pressed()
Images::config_base1()                                 // 变体
```

### 关闭按钮 (Close Button)
```rust
Images::cross_base() / load_cross_base()               // 19x19
Images::cross_hover() / load_cross_hover()
Images::cross_pressed() / load_cross_pressed()
```

### 复选框 (Checkbox)
```rust
Images::checkf_base2() / load_checkf_base2()           // 67x23
Images::checkf_hover() / load_checkf_hover()
Images::checkf_pressed() / load_checkf_pressed()
Images::config_check_off1()                            // 配置复选框-关
Images::config_check_on()                              // 配置复选框-开
```

### 单选框 (Radio Button)
```rust
Images::radio_unactive()                               // 未激活
Images::config_radio_on()                              // 配置单选框-开
```

### 背景和其他 (Background & Others)
```rust
Images::server_base() / load_server_base()             // 186x19 服务器选择
Images::textboxes()                                    // 文本框
Images::pfffft()                                       // 特效图
```

## 常用模式

### 按钮三态
```rust
pub struct ButtonTextures {
    base: DynamicImage,
    hover: DynamicImage,
    pressed: DynamicImage,
}

impl ButtonTextures {
    pub fn launch_button() -> Result<Self, ImageError> {
        Ok(Self {
            base: Images::load_launch_base()?,
            hover: Images::load_launch_hover()?,
            pressed: Images::load_launch_pressed()?,
        })
    }
}
```

### 进度条
```rust
pub struct ProgressBar {
    texture: DynamicImage,
    progress: f32, // 0.0 - 1.0
}

impl ProgressBar {
    pub fn new() -> Result<Self, ImageError> {
        Ok(Self {
            texture: Images::load_blue_progress()?,
            progress: 0.0,
        })
    }
}
```

### 转换为 RGBA8（用于纹理上传）
```rust
let img = Images::load_launch_base()?;
let rgba = img.to_rgba8();
let pixels: &[u8] = rgba.as_raw();
```

### 获取尺寸
```rust
use image::GenericImageView;

let img = Images::load_server_base()?;
let (width, height) = img.dimensions();
```

## 示例

### 查看所有资源信息
```bash
cargo run --example show_resources
```

输出：
```
=== Mir2 Client Embedded Resources ===

📊 Progress Bars:
  Blue Progress: 550x13 pixels
  Green Progress: 550x13 pixels

🔘 Launch Buttons:
  Base State: 115x53 pixels
  ...

📦 Total Embedded Resources:
  Total: 1132 KB (1159331 bytes)
  Resources: 25 PNG images
```

### 按钮状态管理
```bash
cargo run --example button_example
```

## 性能特点

| 特性 | 说明 |
|------|------|
| 加载时机 | 编译时嵌入二进制 |
| 内存占用 | 静态数据段（~1.1 MB） |
| 访问开销 | 零开销（直接引用） |
| 文件 I/O | 无（已嵌入） |
| 线程安全 | 完全安全（静态数据） |

## 集成到项目

### Cargo.toml
```toml
[dependencies]
image = { version = "0.25", default-features = false, features = ["png"] }
```

### 使用
```rust
use mir2_client::resources::Images;
use image::GenericImageView;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let img = Images::load_launch_base()?;
    println!("尺寸: {}x{}", img.width(), img.height());
    Ok(())
}
```

## 测试

```bash
# 运行所有 resources 测试
cargo test --lib resources::tests

# 运行特定测试
cargo test --lib test_load_images_with_image_crate
```

## 注意事项

1. **文件名大小写敏感**: 路径必须完全匹配（包括空格）
2. **编译时检查**: 文件缺失会导致编译错误
3. **二进制大小**: 所有资源嵌入可执行文件（+1.1 MB）
4. **PNG 格式**: 仅支持 PNG 图片

## 故障排除

### 编译错误: "couldn't read ..."
确保 PNG 文件存在于 `resources/ui/` 目录。

### 加载失败
```rust
// 使用 ? 或 match 处理错误
match Images::load_launch_base() {
    Ok(img) => println!("加载成功: {}x{}", img.width(), img.height()),
    Err(e) => eprintln!("加载失败: {}", e),
}
```

## 相关文档

- [完整移植报告](./Resources移植完成报告.md)
- [image crate 文档](https://docs.rs/image/)
- C# 原版: `Client/Resources/Images.Designer.cs`
