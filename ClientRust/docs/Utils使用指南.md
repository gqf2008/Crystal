# Utils 模块快速参考

## 导入

```rust
use mir2_client::utils::{
    open_default_browser,
    open_chrome_browser,
    FileInformation,
    Download,
};
```

## BrowserHelper

### 打开默认浏览器
```rust
open_default_browser("https://www.rust-lang.org")?;
```

### 打开 Chrome
```rust
open_chrome_browser("https://www.rust-lang.org")?;
```

**跨平台**: 自动适配 Windows/macOS/Linux

---

## FileHelper

### FileInformation

#### 创建
```rust
use chrono::Utc;

let info = FileInformation::new(
    "game.pak".to_string(),
    10485760,  // 10 MB 原始
    5242880,   // 5 MB 压缩
    Utc::now(),
);
```

#### 序列化
```rust
let mut buffer = Vec::new();
info.write_to(&mut buffer)?;
```

#### 反序列化
```rust
use std::io::Cursor;

let info = FileInformation::read_from(&mut Cursor::new(buffer))?;
```

---

### Download

#### 创建
```rust
let download = Download::new(file_info);
```

#### 更新进度
```rust
download.update_progress(5242880);  // 已下载字节数
```

#### 获取进度
```rust
let progress = download.progress();          // 0.0 - 1.0
let percent = download.progress_percent();   // 0 - 100
let is_done = download.completed;            // bool
```

---

## 完整示例

### 多文件下载
```rust
use mir2_client::utils::{FileInformation, Download};
use chrono::Utc;

// 创建文件列表
let files = vec![
    ("maps.dat", 20971520, 10485760),
    ("textures.pak", 52428800, 26214400),
    ("sounds.pak", 31457280, 15728640),
];

// 创建下载跟踪器
let mut downloads: Vec<Download> = files
    .iter()
    .map(|(name, len, comp)| {
        let info = FileInformation::new(
            name.to_string(),
            *len,
            *comp,
            Utc::now(),
        );
        Download::new(info)
    })
    .collect();

// 更新进度
downloads[0].update_progress(20971520);  // 100%
downloads[1].update_progress(13107200);  // 25%

// 显示进度
for dl in &downloads {
    println!("[{}%] {}", 
             dl.progress_percent(), 
             dl.info.file_name);
}
```

---

## 测试

```bash
# 运行所有测试
cargo test --lib utils::

# 运行示例
cargo run --example file_helper_example
cargo run --example browser_helper_example
```

---

## 注意事项

1. **浏览器测试**: 默认 `#[ignore]`，手动运行需移除
2. **DateTime 兼容**: 与 .NET `DateTime.ToBinary()` 完全兼容
3. **字节序**: 使用 Little-Endian（与 .NET 一致）
4. **编码**: 字符串使用 UTF-8

---

## 相关文档

- [完整移植报告](./Utils移植完成报告.md)
- [BrowserHelper 源码](../src/utils/browser_helper.rs)
- [FileHelper 源码](../src/utils/file_helper.rs)
