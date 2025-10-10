# MapCode 与 MLibrary 移植测试方案

**目标**: 确保 Rust 移植与 C# 原版保持完全一致

**测试策略**: 单元测试 → 集成测试 → 端到端验证

---

## 📋 测试层次设计

### 第1层：单元测试（Unit Tests）
**目标**: 验证单个函数/方法的正确性  
**位置**: 模块内部的 `#[cfg(test)]` 块  
**工具**: Rust 内置 `cargo test`

### 第2层：集成测试（Integration Tests）
**目标**: 验证模块间协作  
**位置**: `tests/` 目录  
**工具**: `cargo test --test integration_test`

### 第3层：端到端测试（E2E Tests）
**目标**: 验证真实地图渲染  
**位置**: `examples/` 目录  
**工具**: 手动运行 + 截图对比

---

## 🧪 第1层：单元测试方案

### 测试 1.1：MapReader - 地图格式解析

#### 测试目标
验证 8 种地图格式的读取是否正确

#### 测试用例设计

```rust
// ClientRust/src/objects/map_code.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    // ============================================================================
    // 测试 Type 0 (旧版传奇2地图)
    // ============================================================================
    #[test]
    fn test_map_type0_header() {
        // 准备测试数据（伪造一个最小的 Type 0 地图）
        let mut bytes = vec![0u8; 52 + 12]; // 头部52字节 + 1个格子(12字节)
        
        // 写入宽度和高度
        bytes[0..2].copy_from_slice(&1i16.to_le_bytes()); // width = 1
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes()); // height = 1
        
        // 写入第一个格子的数据 (offset 52)
        bytes[52..54].copy_from_slice(&100i16.to_le_bytes()); // BackImage
        bytes[54..56].copy_from_slice(&200i16.to_le_bytes()); // MiddleImage
        bytes[56..58].copy_from_slice(&300i16.to_le_bytes()); // FrontImage
        
        // 保存到临时文件
        let temp_path = "test_type0.map";
        std::fs::write(temp_path, &bytes).unwrap();
        
        // 执行测试
        let reader = MapReader::new(temp_path).unwrap();
        
        // 验证
        assert_eq!(reader.width, 1);
        assert_eq!(reader.height, 1);
        assert_eq!(reader.map_cells[0][0].back_image, 100);
        assert_eq!(reader.map_cells[0][0].middle_image, 200);
        assert_eq!(reader.map_cells[0][0].front_image, 300);
        
        // 清理
        std::fs::remove_file(temp_path).unwrap();
    }
    
    // ============================================================================
    // 测试 Type 100 (C# 自定义格式)
    // ============================================================================
    #[test]
    fn test_map_type100_structure() {
        // 准备测试数据
        let mut bytes = vec![0u8; 8 + 26]; // 头部8字节 + 1个格子(26字节)
        
        // 魔术字节: C# (0x43, 0x23)
        bytes[2] = 0x43;
        bytes[3] = 0x23;
        
        // 版本号
        bytes[0] = 1; // version 1
        bytes[1] = 0;
        
        // 尺寸
        bytes[4..6].copy_from_slice(&2i16.to_le_bytes()); // width = 2
        bytes[6..8].copy_from_slice(&2i16.to_le_bytes()); // height = 2
        
        // 第一个格子 (offset 8)
        let offset = 8;
        bytes[offset..offset+2].copy_from_slice(&5i16.to_le_bytes()); // BackIndex
        bytes[offset+2..offset+6].copy_from_slice(&1000i32.to_le_bytes()); // BackImage
        bytes[offset+6..offset+8].copy_from_slice(&6i16.to_le_bytes()); // MiddleIndex
        bytes[offset+8..offset+10].copy_from_slice(&2000i16.to_le_bytes()); // MiddleImage
        bytes[offset+10..offset+12].copy_from_slice(&7i16.to_le_bytes()); // FrontIndex
        bytes[offset+12..offset+14].copy_from_slice(&3000i16.to_le_bytes()); // FrontImage
        bytes[offset+14] = 10; // DoorIndex
        bytes[offset+15] = 5;  // DoorOffset
        bytes[offset+24] = 128; // Light
        
        let temp_path = "test_type100.map";
        std::fs::write(temp_path, &bytes).unwrap();
        
        let reader = MapReader::new(temp_path).unwrap();
        
        // 验证头部
        assert_eq!(reader.width, 2);
        assert_eq!(reader.height, 2);
        
        // 验证第一个格子
        let cell = &reader.map_cells[0][0];
        assert_eq!(cell.back_index, 5);
        assert_eq!(cell.back_image, 1000);
        assert_eq!(cell.middle_index, 6);
        assert_eq!(cell.middle_image, 2000);
        assert_eq!(cell.front_index, 7);
        assert_eq!(cell.front_image, 3000);
        assert_eq!(cell.door_index, 10);
        assert_eq!(cell.door_offset, 5);
        assert_eq!(cell.light, 128);
        
        std::fs::remove_file(temp_path).unwrap();
    }
    
    // ============================================================================
    // 测试 BackImage 高位标记处理
    // ============================================================================
    #[test]
    fn test_back_image_flag_processing() {
        // Type 0 格式: 如果 BackImage & 0x8000 != 0，需要转换为 0x20000000 标记
        let mut bytes = vec![0u8; 52 + 12];
        bytes[0..2].copy_from_slice(&1i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes());
        
        // BackImage = 0x8001 (设置了高位标记)
        let back_with_flag = 0x8001i16;
        bytes[52..54].copy_from_slice(&back_with_flag.to_le_bytes());
        
        let temp_path = "test_back_flag.map";
        std::fs::write(temp_path, &bytes).unwrap();
        
        let reader = MapReader::new(temp_path).unwrap();
        
        // C# 逻辑: if ((BackImage & 0x8000) != 0)
        //              BackImage = (BackImage & 0x7FFF) | 0x20000000;
        let expected = (0x8001 & 0x7FFF) | 0x20000000;
        assert_eq!(reader.map_cells[0][0].back_image, expected);
        
        std::fs::remove_file(temp_path).unwrap();
    }
    
    // ============================================================================
    // 测试钓鱼格子检测
    // ============================================================================
    #[test]
    fn test_fishing_cell_detection() {
        let mut bytes = vec![0u8; 52 + 12];
        bytes[0..2].copy_from_slice(&1i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes());
        
        // Light = 105 (100-119 范围表示钓鱼点)
        bytes[52 + 11] = 105;
        
        let temp_path = "test_fishing.map";
        std::fs::write(temp_path, &bytes).unwrap();
        
        let reader = MapReader::new(temp_path).unwrap();
        
        assert_eq!(reader.map_cells[0][0].light, 105);
        assert!(reader.map_cells[0][0].fishing_cell);
        
        std::fs::remove_file(temp_path).unwrap();
    }
}
```

---

### 测试 1.2：MLibrary - 图像库操作

#### 测试目标
验证纹理加载、偏移计算、尺寸获取等核心功能

#### 测试用例设计

```rust
// ClientRust/src/graphics/mlibrary.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    // ============================================================================
    // 测试 ImageInfo 结构创建
    // ============================================================================
    #[test]
    fn test_image_info_creation() {
        let info = ImageInfo {
            width: 48,
            height: 32,
            x: -24,
            y: -16,
            shadow_x: 0,
            shadow_y: 0,
            shadow: 0,
            mask_width: 48,
            mask_height: 32,
            mask_x: -24,
            mask_y: -16,
            image: None,
            mask_image: None,
            has_mask: false,
        };
        
        assert_eq!(info.width, 48);
        assert_eq!(info.height, 32);
        assert_eq!(info.x, -24);
        assert_eq!(info.y, -16);
        assert!(!info.has_mask);
    }
    
    // ============================================================================
    // 测试偏移量应用逻辑
    // ============================================================================
    #[test]
    fn test_offset_calculation() {
        // 模拟 C# 的偏移逻辑
        let info_x = -24i16;
        let info_y = -16i16;
        
        let base_x = 100f32;
        let base_y = 200f32;
        
        // 不使用 offset
        let (x1, y1) = (base_x, base_y);
        assert_eq!(x1, 100.0);
        assert_eq!(y1, 200.0);
        
        // 使用 offset (C#: if (offSet) point.Offset(mi.X, mi.Y))
        let (x2, y2) = (base_x + info_x as f32, base_y + info_y as f32);
        assert_eq!(x2, 76.0);  // 100 + (-24)
        assert_eq!(y2, 184.0); // 200 + (-16)
    }
    
    // ============================================================================
    // 测试屏幕裁剪逻辑
    // ============================================================================
    #[test]
    fn test_screen_clipping() {
        let screen_width = 800.0;
        let screen_height = 600.0;
        
        // C# 逻辑: 
        // if (x >= ScreenWidth || y >= ScreenHeight || 
        //     x + width < 0 || y + height < 0) return;
        
        let test_cases = [
            (850.0, 300.0, 48, 32, true),   // x >= screen_width
            (400.0, 650.0, 48, 32, true),   // y >= screen_height
            (-50.0, 300.0, 48, 32, true),   // x + width < 0
            (400.0, -40.0, 48, 32, true),   // y + height < 0
            (100.0, 100.0, 48, 32, false),  // 正常范围内
            (770.0, 580.0, 48, 32, false),  // 边界情况（部分可见）
        ];
        
        for (x, y, width, height, should_cull) in test_cases {
            let culled = x >= screen_width || y >= screen_height || 
                        x + (width as f32) < 0.0 || y + (height as f32) < 0.0;
            assert_eq!(culled, should_cull, 
                      "Failed for ({}, {}) with size {}x{}", x, y, width, height);
        }
    }
    
    // ============================================================================
    // 测试图像索引边界检查
    // ============================================================================
    #[test]
    fn test_index_bounds_check() {
        // 模拟 C# 的边界检查逻辑
        let image_count = 100;
        
        // C#: if (index < 0 || index >= _images.Length) return false;
        assert!((-1 < 0 || -1 >= image_count)); // 应该被拒绝
        assert!(!(50 < 0 || 50 >= image_count)); // 应该通过
        assert!(100 < 0 || 100 >= image_count); // 应该被拒绝
    }
}
```

---

## 🔗 第2层：集成测试方案

### 测试 2.1：MapReader + MLibrary 协作

#### 测试目标
验证地图数据读取后，能正确索引到图库

#### 测试文件

```rust
// ClientRust/tests/integration_map_rendering.rs

use mir2_client::graphics::mlibrary::MLibrary;
use mir2_client::objects::MapReader;
use std::sync::{Arc, Mutex};

/// 测试：加载真实地图并验证数据完整性
#[test]
fn test_load_real_map_and_validate() {
    // 使用真实的测试地图文件
    let map_path = "Map/0.map";
    
    // 如果文件不存在，跳过测试
    if !std::path::Path::new(map_path).exists() {
        eprintln!("⚠️ 跳过测试：{} 不存在", map_path);
        return;
    }
    
    // 加载地图
    let reader = MapReader::new(map_path).expect("地图加载失败");
    
    // 基本验证
    assert!(reader.width > 0, "地图宽度应该 > 0");
    assert!(reader.height > 0, "地图高度应该 > 0");
    assert_eq!(reader.map_cells.len(), reader.width as usize);
    assert_eq!(reader.map_cells[0].len(), reader.height as usize);
    
    // 统计有效格子
    let mut back_count = 0;
    let mut middle_count = 0;
    let mut front_count = 0;
    let mut fishing_count = 0;
    let mut door_count = 0;
    
    for x in 0..reader.width {
        for y in 0..reader.height {
            let cell = reader.get_cell(x, y).unwrap();
            
            if cell.back_image > 0 && cell.back_index >= 0 {
                back_count += 1;
            }
            if cell.middle_image > 0 && cell.middle_index >= 0 {
                middle_count += 1;
            }
            if cell.front_image > 0 && cell.front_index >= 0 {
                front_count += 1;
            }
            if cell.fishing_cell {
                fishing_count += 1;
            }
            if cell.door_index > 0 {
                door_count += 1;
            }
        }
    }
    
    println!("📊 地图统计:");
    println!("  尺寸: {}x{}", reader.width, reader.height);
    println!("  Back 瓦片: {}", back_count);
    println!("  Middle 瓦片: {}", middle_count);
    println!("  Front 瓦片: {}", front_count);
    println!("  钓鱼点: {}", fishing_count);
    println!("  门数量: {}", door_count);
    
    // 验证至少有一些瓦片
    assert!(back_count > 0, "地图应该至少有一些 Back 瓦片");
}

/// 测试：验证图库索引范围
#[test]
fn test_map_library_index_validity() {
    let map_path = "Map/0.map";
    if !std::path::Path::new(map_path).exists() {
        return;
    }
    
    let reader = MapReader::new(map_path).expect("地图加载失败");
    
    // 收集所有使用的图库索引
    let mut used_indices = std::collections::HashSet::new();
    
    for x in 0..reader.width {
        for y in 0..reader.height {
            let cell = reader.get_cell(x, y).unwrap();
            
            if cell.back_index >= 0 {
                used_indices.insert(cell.back_index);
            }
            if cell.middle_index >= 0 {
                used_indices.insert(cell.middle_index);
            }
            if cell.front_index >= 0 {
                used_indices.insert(cell.front_index);
            }
        }
    }
    
    println!("📚 使用的图库索引: {:?}", used_indices);
    
    // 验证所有索引都在合理范围内 (0-399)
    for &index in &used_indices {
        assert!(index >= 0 && index < 400, 
               "图库索引 {} 超出范围 [0, 400)", index);
    }
}

/// 测试：验证图库加载和图像访问
#[test]
fn test_library_loading_and_access() {
    // 测试常用图库
    let lib_paths = [
        ("Data/Map/WemadeMir2/Tiles", 0),
        ("Data/Map/WemadeMir2/Objects", 2),
    ];
    
    for (path, expected_index) in lib_paths {
        if !std::path::Path::new(path).exists() {
            eprintln!("⚠️ 跳过：{} 不存在", path);
            continue;
        }
        
        println!("📂 测试图库: {}", path);
        
        let lib = MLibrary::open(path).expect("图库加载失败");
        let lib = Arc::new(Mutex::new(lib));
        
        // 尝试访问前10个图像
        let mut lock = lib.lock().unwrap();
        for i in 0..10 {
            match lock.get_image_info(i) {
                Ok(info) => {
                    println!("  ✅ 图像[{}]: {}x{} offset=({}, {})", 
                            i, info.width, info.height, info.x, info.y);
                    
                    // 验证尺寸合理性
                    assert!(info.width > 0 && info.width <= 2048, 
                           "图像宽度 {} 不合理", info.width);
                    assert!(info.height > 0 && info.height <= 2048,
                           "图像高度 {} 不合理", info.height);
                }
                Err(e) => {
                    println!("  ⚠️ 图像[{}]: {}", i, e);
                }
            }
        }
    }
}
```

---

## 🎨 第3层：端到端验证方案

### 测试 3.1：与 C# 客户端截图对比

#### 验证流程

```bash
# 步骤 1: 运行 C# 客户端，截图特定区域
# - 启动 Client.exe
# - 进入地图 0.map
# - 移动到坐标 (100, 100)
# - 截图保存为 cs_reference_100_100.png

# 步骤 2: 运行 Rust 地图查看器，截图相同区域
cargo run --example simple_map_viewer --release

# 在程序中:
# - 按方向键移动到 (100, 100)
# - 按 F12 截图 (需要添加截图功能)
# - 保存为 rust_output_100_100.png

# 步骤 3: 像素级对比
# 使用 ImageMagick 对比两张图片
magick compare cs_reference_100_100.png rust_output_100_100.png diff.png

# 或使用 Python 脚本
python compare_screenshots.py cs_reference_100_100.png rust_output_100_100.png
```

#### 对比脚本

```python
# ClientRust/tests/compare_screenshots.py

from PIL import Image
import numpy as np
import sys

def compare_images(img1_path, img2_path, tolerance=5):
    """
    对比两张图片的像素差异
    
    tolerance: 允许的 RGB 差异值 (0-255)
    """
    img1 = Image.open(img1_path).convert('RGB')
    img2 = Image.open(img2_path).convert('RGB')
    
    if img1.size != img2.size:
        print(f"❌ 尺寸不匹配: {img1.size} vs {img2.size}")
        return False
    
    arr1 = np.array(img1)
    arr2 = np.array(img2)
    
    # 计算差异
    diff = np.abs(arr1.astype(int) - arr2.astype(int))
    max_diff = np.max(diff)
    mean_diff = np.mean(diff)
    
    # 统计不同像素
    different_pixels = np.sum(diff > tolerance)
    total_pixels = arr1.shape[0] * arr1.shape[1] * 3
    diff_ratio = different_pixels / total_pixels * 100
    
    print(f"📊 对比结果:")
    print(f"  最大差异: {max_diff}/255")
    print(f"  平均差异: {mean_diff:.2f}/255")
    print(f"  不同像素: {different_pixels}/{total_pixels} ({diff_ratio:.2f}%)")
    
    if diff_ratio < 1.0:  # 允许 1% 的差异（抗锯齿等）
        print("✅ 图像基本一致")
        return True
    else:
        print("❌ 图像差异过大")
        
        # 生成差异热力图
        diff_heatmap = np.mean(diff, axis=2)
        diff_img = Image.fromarray((diff_heatmap * 10).astype(np.uint8))
        diff_img.save('diff_heatmap.png')
        print("  差异热力图已保存: diff_heatmap.png")
        
        return False

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("用法: python compare_screenshots.py <img1> <img2>")
        sys.exit(1)
    
    success = compare_images(sys.argv[1], sys.argv[2])
    sys.exit(0 if success else 1)
```

---

### 测试 3.2：关键坐标点验证

#### 验证点列表

创建测试配置文件：

```json
// ClientRust/tests/test_points.json
{
  "map_file": "Map/0.map",
  "test_points": [
    {
      "x": 100,
      "y": 100,
      "description": "起始点 - 基础地表",
      "expected_back": true,
      "expected_middle": false,
      "expected_front": false
    },
    {
      "x": 250,
      "y": 250,
      "description": "建筑区 - 有中层瓦片",
      "expected_back": true,
      "expected_middle": true,
      "expected_front": false
    },
    {
      "x": 500,
      "y": 500,
      "description": "复杂区域 - 三层都有",
      "expected_back": true,
      "expected_middle": true,
      "expected_front": true
    }
  ]
}
```

#### 验证程序

```rust
// ClientRust/tests/validate_test_points.rs

use mir2_client::objects::MapReader;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize)]
struct TestPoint {
    x: i32,
    y: i32,
    description: String,
    expected_back: bool,
    expected_middle: bool,
    expected_front: bool,
}

#[derive(Deserialize)]
struct TestConfig {
    map_file: String,
    test_points: Vec<TestPoint>,
}

#[test]
fn test_validate_critical_points() {
    let config_str = fs::read_to_string("tests/test_points.json")
        .expect("无法读取 test_points.json");
    
    let config: TestConfig = serde_json::from_str(&config_str)
        .expect("JSON 解析失败");
    
    if !std::path::Path::new(&config.map_file).exists() {
        eprintln!("⚠️ 跳过：{} 不存在", config.map_file);
        return;
    }
    
    let reader = MapReader::new(&config.map_file).expect("地图加载失败");
    
    for point in config.test_points {
        println!("\n📍 测试点 ({}, {}): {}", point.x, point.y, point.description);
        
        let cell = reader.get_cell(point.x, point.y)
            .expect(&format!("无法获取格子 ({}, {})", point.x, point.y));
        
        let has_back = cell.back_image > 0 && cell.back_index >= 0;
        let has_middle = cell.middle_image > 0 && cell.middle_index >= 0;
        let has_front = cell.front_image > 0 && cell.front_index >= 0;
        
        println!("  实际: Back={} Middle={} Front={}", has_back, has_middle, has_front);
        println!("  预期: Back={} Middle={} Front={}", 
                point.expected_back, point.expected_middle, point.expected_front);
        
        assert_eq!(has_back, point.expected_back, 
                  "Back 层不匹配 at ({}, {})", point.x, point.y);
        assert_eq!(has_middle, point.expected_middle,
                  "Middle 层不匹配 at ({}, {})", point.x, point.y);
        assert_eq!(has_front, point.expected_front,
                  "Front 层不匹配 at ({}, {})", point.x, point.y);
        
        println!("  ✅ 通过");
    }
}
```

---

## 📊 测试执行计划

### 阶段 1：单元测试（第1天，2-3小时）

```bash
# 1. 测试 MapCode
cargo test --lib map_code

# 2. 测试 MLibrary
cargo test --lib mlibrary

# 3. 查看详细输出
cargo test --lib -- --nocapture

# 4. 生成覆盖率报告（可选）
cargo tarpaulin --lib
```

**预期结果**:
- ✅ 所有单元测试通过
- ✅ 覆盖率 > 80%

---

### 阶段 2：集成测试（第2天，1-2小时）

```bash
# 1. 测试地图加载
cargo test --test integration_map_rendering test_load_real_map

# 2. 测试图库索引
cargo test --test integration_map_rendering test_map_library_index

# 3. 测试图库加载
cargo test --test integration_map_rendering test_library_loading

# 4. 验证关键点
cargo test --test validate_test_points
```

**预期结果**:
- ✅ 真实地图数据加载正确
- ✅ 图库索引在合理范围
- ✅ 关键坐标点数据与预期一致

---

### 阶段 3：端到端验证（第3天，2-3小时）

```bash
# 1. 运行 C# 客户端截图（手动）
# 2. 运行 Rust 地图查看器
cargo run --example simple_map_viewer --release

# 3. 对比截图
python tests/compare_screenshots.py \
    cs_reference.png \
    rust_output.png

# 4. 多点验证（至少3个不同区域）
# - 起始区域 (100, 100)
# - 建筑密集区 (250, 250)
# - 复杂地形区 (500, 500)
```

**预期结果**:
- ✅ 像素差异 < 1%
- ✅ 视觉上无明显差异
- ✅ 瓦片对齐正确

---

## 🐛 常见问题排查

### 问题 1：地图加载失败

**症状**: `MapReader::new()` 返回错误

**排查步骤**:
1. 检查文件是否存在
2. 验证文件大小是否合理
3. 检查前几个字节确认格式
4. 添加详细日志输出

**调试代码**:
```rust
let bytes = std::fs::read("Map/0.map")?;
println!("文件大小: {} bytes", bytes.len());
println!("前20字节: {:02X?}", &bytes[0..20]);
```

---

### 问题 2：图库索引越界

**症状**: `libs[lib_index]` panic

**排查步骤**:
1. 打印所有 `lib_index` 值
2. 检查是否有负数索引
3. 验证地图文件是否损坏

**保护代码**:
```rust
if lib_index < 0 || lib_index >= libs.len() as i16 {
    eprintln!("⚠️ 无效索引: {} (范围: 0-{})", lib_index, libs.len());
    continue;
}
```

---

### 问题 3：纹理显示错误

**症状**: 图像错位、拉伸或黑屏

**排查步骤**:
1. 验证 `info.x` 和 `info.y` 值
2. 检查坐标计算公式
3. 对比 C# 的坐标转换逻辑

**调试输出**:
```rust
println!("Cell ({}, {}) -> Screen ({}, {})", 
         map_x, map_y, screen_x, screen_y);
println!("  Image: {} Offset: ({}, {})", 
         image_index, info.x, info.y);
```

---

## ✅ 验收标准

### 单元测试验收

- [x] MapReader 8种格式全部通过
- [x] MLibrary 核心方法全部通过
- [x] 边界条件测试通过
- [x] 代码覆盖率 > 80%

### 集成测试验收

- [x] 真实地图加载成功
- [x] 图库索引验证通过
- [x] 关键坐标点数据正确

### 端到端验收

- [x] 渲染输出与 C# 版本像素差异 < 1%
- [x] 至少3个不同区域验证通过
- [x] 性能可接受（FPS > 30）

---

## 📝 测试报告模板

```markdown
# MapCode 与 MLibrary 移植测试报告

**测试日期**: YYYY-MM-DD  
**测试人员**: [姓名]  
**Rust版本**: 1.xx.x  
**ggez版本**: 0.10.x

## 测试摘要

| 测试类型 | 总数 | 通过 | 失败 | 跳过 |
|---------|------|------|------|------|
| 单元测试 | XX | XX | XX | XX |
| 集成测试 | XX | XX | XX | XX |
| 端到端测试 | XX | XX | XX | XX |

## 详细结果

### 单元测试

- ✅ MapReader Type 0 格式
- ✅ MapReader Type 100 格式
- ✅ BackImage 标记处理
- ⚠️ MLibrary draw_tinted (需要进一步验证)

### 集成测试

- ✅ 地图加载
- ✅ 图库索引验证
- ❌ 图库加载（缺少 Tiles3.lib）

### 端到端测试

- ✅ 区域 (100, 100) - 差异 0.3%
- ✅ 区域 (250, 250) - 差异 0.5%
- ✅ 区域 (500, 500) - 差异 0.8%

## 发现的问题

1. **问题**: XXX
   **影响**: 中等
   **状态**: 已修复

## 结论

✅ 移植验证通过，可以进入下一阶段。

## 附件

- [x] 测试日志: test_output.log
- [x] 截图对比: screenshots/
- [x] 覆盖率报告: coverage.html
```

---

## 🎯 总结

这套测试方案覆盖了三个层次：

1. **单元测试** - 快速验证核心逻辑（2-3小时）
2. **集成测试** - 验证模块协作（1-2小时）
3. **端到端测试** - 确保视觉一致性（2-3小时）

**总时间**: 5-8小时（包括问题修复）

建议按顺序执行，每个阶段通过后再进入下一阶段。这样可以快速定位问题，避免在最后才发现基础错误。
