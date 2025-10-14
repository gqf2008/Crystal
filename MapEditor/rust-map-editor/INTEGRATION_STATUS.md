# 库文件整合状态

## 已完成的整合工作

### 1. 模块导入更新 ✅
- 更新 `main.rs` 使用 `mlibrary` 和 `map_code` 模块
- 修复 `mlibrary.rs` 的导入路径：`crate::objects::frames` → `crate::frames`
- 修复 `mlibrary.rs` 的 MirAction 导入：`mir2_shared::MirAction` → `mir2_shared::enums::MirAction`
- 注释掉 `map_code.rs` 中暂时不需要的 `DrawableMapObject` 导入

### 2. 依赖更新 ✅
在 `Cargo.toml` 中添加了缺失的依赖：
- `flate2 = "1.0"` - 用于 GZ 解压缩
- `glam = "0.27"` - 用于数学运算

### 3. EditorState 重构 ✅
- 将 `LibraryManager` 改为 `Vec<MLibrary>`
- 将 `MapReader` 改为 `Option<MapReader>` 以支持延迟加载
- 更新所有相关方法处理 Option 类型
- 修复地图加载和保存方法

### 4. Renderer 更新 ✅
- 更新导入使用 `map_code::CellInfo` 和 `map_code::MapReader`
- 更新导入使用 `mlibrary::MLibrary`
- 修复渲染方法中访问 map_cells 的方式
- 直接使用 `map.map_cells[x][y]` 而不是 `map.get_cell()`
- 修复位掩码操作以获取图片索引

## 当前项目结构

```
src/
├── main.rs              # 程序入口
├── mlibrary.rs          # ✅ 你的库文件解析实现
├── map_code.rs          # ✅ 你的地图解析实现
├── frames.rs            # ✅ 动画帧管理
├── renderer.rs          # ✅ 已更新的渲染器
├── editor_state.rs      # ✅ 已更新的编辑器状态
├── cell_info.rs         # ⚠️  可能已被 map_code.rs 替代
├── map_reader.rs        # ⚠️  可能已被 map_code.rs 替代
└── library.rs           # ⚠️  可能已被 mlibrary.rs 替代
```

## 待处理的事项

### 高优先级 🔴

1. **编译验证**
   ```bash
   cargo check
   cargo build
   ```

2. **移除冗余文件**
   - `cell_info.rs` (已被 `map_code.rs` 中的 CellInfo 替代)
   - `map_reader.rs` (已被 `map_code.rs` 中的 MapReader 替代)
   - `library.rs` (已被 `mlibrary.rs` 替代)

3. **修复可能的编译错误**
   - 检查 `map_code.rs` 中所有使用 `DrawableMapObject` 的地方
   - 确认所有位操作正确 (例如: `back_image & 0x1FFFFFFF`)
   - 验证数据类型匹配 (i16, i32, u8 等)

### 中优先级 🟡

4. **实现实际图片渲染**
   ```rust
   // 在 renderer.rs 中:
   // 1. 加载 MLibrary 纹理
   // 2. 根据 cell 的 back_index, middle_index, front_index 获取纹理
   // 3. 使用 ggez 的 Image 绘制纹理
   ```

5. **实现库管理器**
   ```rust
   // 在 editor_state.rs 或新建 library_manager.rs:
   pub struct LibraryManager {
       libraries: HashMap<i16, MLibrary>,
   }
   
   impl LibraryManager {
       pub fn load_library(&mut self, ctx: &mut Context, index: i16, path: &str) -> GameResult {
           let mut lib = MLibrary::new();
           lib.load(path)?;
           self.libraries.insert(index, lib);
           Ok(())
       }
       
       pub fn get_image(&mut self, ctx: &mut Context, lib_index: i16, img_index: i32) -> Option<&ggez::graphics::Image> {
           if let Some(lib) = self.libraries.get_mut(&lib_index) {
               lib.get_image(ctx, img_index as usize)
           } else {
               None
           }
       }
   }
   ```

6. **实现动画系统**
   - 使用 `animation_count` 和 `AnimationFrame/AnimationTick` 计算当前帧
   - 参考 C# 原版的动画逻辑

### 低优先级 🟢

7. **UI 界面**
   - 考虑使用 `egui` 集成
   - 图块选择器
   - 属性面板
   - 菜单栏

8. **高级编辑功能**
   - 图块放置/删除
   - 对象编辑
   - 门和光照编辑
   - 撤销/重做

9. **地图保存**
   - 实现 MapReader 的保存功能
   - 支持多种格式导出

## 测试步骤

### 1. 编译测试
```bash
cd rust-map-editor
cargo clean
cargo check
cargo build --release
```

### 2. 运行测试
```bash
cargo run --release
```

### 3. 测试地图加载
修改 `main.rs` 或添加命令行参数加载测试地图：
```rust
// 在 EditorState::new 中
pub fn new(ctx: &mut Context) -> GameResult<Self> {
    let mut state = EditorState { ... };
    
    // 尝试加载测试地图
    if let Err(e) = state.load_map(ctx, "./test.map") {
        println!("No test map found: {:?}", e);
    }
    
    Ok(state)
}
```

## 已知问题

1. **DrawableMapObject 未定义**
   - 暂时注释掉
   - 需要决定是否需要这个类型

2. **mir2_shared 依赖**
   - 需要确保 `../SharedRust` 路径正确
   - 需要 `client-parse` feature

3. **数据类型不匹配**
   - CellInfo 中 middle_image 是 i16 还是 i32?
   - 需要统一所有类型定义

## 下一步建议

1. **立即执行**: 运行 `cargo check` 查看编译错误
2. **短期目标**: 实现基本的图片加载和渲染
3. **中期目标**: 完成编辑功能
4. **长期目标**: UI 和高级功能

## 参考资料

- 原 C# 代码: `Map Editor/Main.cs`
- 原库管理: `Map Editor/MLibrary.cs`
- 原地图代码: `Map Editor/MapCode.cs`
