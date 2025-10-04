# Phase 2 重新规划：正确移植 MirGraphics

**日期**: 2025-01-04  
**状态**: ❌ **之前的方案作废，重新开始**

---

## ❌ 错误方案总结

### 我犯的错误

1. **创造了不存在的 Renderer trait**
   - C# 里没有 Renderer 接口
   - DXManager 是静态类，直接调用
   - 我错误地引入了过度抽象

2. **在 objects 模块创建 renderer.rs**
   - Renderer 不属于 MirObjects
   - 应该在 graphics 模块

3. **没有严格按照 C# 结构**
   - 应该先移植 DXManager
   - 然后移植 MLibrary
   - 最后才是 PlayerObject 调用

---

## ✅ 正确方案：严格移植 C# 结构

### C# 结构分析

**Client/MirGraphics/**:
- `DXManager.cs` - 静态类，管理 DirectX 设备和渲染
  - `static Device Device`
  - `static Sprite Sprite`
  - `static void Draw(Texture, Rectangle?, Vector3?, Color4)`
  - `static void DrawOpaque(Texture, Rectangle?, Vector3?, Color4, float opacity)`
  - `static void SetOpacity(float opacity)`
  - `static void SetGrayscale(bool value)`
  - `static void SetBlend(bool value, float rate, BlendMode mode)`

- `MLibrary.cs` - 纹理库类
  - `public void Draw(int index, Point point, Color colour, bool offSet)`
  - `public void DrawBlend(int index, Point point, Color colour, bool offSet, float rate)`
  - 内部调用 `DXManager.Draw()` 和 `DXManager.DrawOpaque()`

**Client/MirObjects/PlayerObject.cs**:
- `public override void Draw()` - 调用各个部位的绘制方法
- `DrawBody()`, `DrawHead()`, `DrawWeapon()` 等 - 调用 `MLibrary.Draw()`

---

## 🎯 正确的移植步骤

### Step 1: 创建 DXManager (Rust 版本)

**新文件**: `ClientRust/src/graphics/dx_manager.rs`

由于 Rust 不用 DirectX，我们用 egui + wgpu 替代，但**保持 API 一致**：

```rust
// dx_manager.rs
// 对应 Client/MirGraphics/DXManager.cs
//
// 注意：Rust 版本使用 egui + wgpu 替代 DirectX 9
// 但保持与 C# 相同的 API 设计

use egui::{Context, ColorImage, TextureHandle, Color32, Pos2, Rect};
use std::collections::HashMap;

/// DXManager - 图形设备管理器
/// 
/// C# equivalent: Client.MirGraphics.DXManager
/// 
/// 与 C# 不同之处：
/// - 使用 egui + wgpu 替代 SlimDX (DirectX 9)
/// - 非静态实现（Rust 最佳实践）
/// - 但 API 设计保持一致
pub struct DXManager {
    /// egui context (替代 C# 的 Device)
    ctx: Context,
    
    /// 纹理缓存 (替代 C# 的 TextureList)
    texture_cache: HashMap<String, TextureHandle>,
    
    /// 当前透明度 (对应 C# 的 Opacity)
    opacity: f32,
    
    /// 是否启用混合模式 (对应 C# 的 Blending)
    blending: bool,
    
    /// 混合率 (对应 C# 的 BlendingRate)
    blending_rate: f32,
    
    /// 灰度模式 (对应 C# 的 GrayScale)
    grayscale: bool,
    
    /// 屏幕宽度
    screen_width: u32,
    
    /// 屏幕高度
    screen_height: u32,
}

impl DXManager {
    /// 创建 DXManager
    /// 
    /// C# equivalent: DXManager.Create()
    pub fn new(ctx: Context, screen_width: u32, screen_height: u32) -> Self {
        Self {
            ctx,
            texture_cache: HashMap::new(),
            opacity: 1.0,
            blending: false,
            blending_rate: 1.0,
            grayscale: false,
            screen_width,
            screen_height,
        }
    }
    
    /// 绘制纹理
    /// 
    /// C# equivalent: DXManager.Draw(Texture, Rectangle?, Vector3?, Color4)
    /// 
    /// # Arguments
    /// 
    /// * `texture_handle` - 纹理句柄
    /// * `source_rect` - 源矩形（纹理的哪部分）
    /// * `position` - 屏幕位置
    /// * `color` - 颜色（RGBA，0xRRGGBBAA 格式）
    pub fn draw(
        &self,
        ui: &mut egui::Ui,
        texture_handle: &TextureHandle,
        source_rect: Option<Rect>,
        position: Pos2,
        color: u32,
    ) {
        // 将 u32 颜色转换为 Color32
        let r = ((color >> 24) & 0xFF) as u8;
        let g = ((color >> 16) & 0xFF) as u8;
        let b = ((color >> 8) & 0xFF) as u8;
        let a = (color & 0xFF) as u8;
        
        // 应用全局透明度
        let final_alpha = (a as f32 * self.opacity) as u8;
        let tint = Color32::from_rgba_unmultiplied(r, g, b, final_alpha);
        
        // TODO: 实际绘制逻辑
        // 这里需要使用 egui::Image 或自定义渲染
    }
    
    /// 绘制带透明度的纹理
    /// 
    /// C# equivalent: DXManager.DrawOpaque(Texture, Rectangle?, Vector3?, Color4, float)
    pub fn draw_opaque(
        &self,
        ui: &mut egui::Ui,
        texture_handle: &TextureHandle,
        source_rect: Option<Rect>,
        position: Pos2,
        color: u32,
        opacity: f32,
    ) {
        // 临时修改透明度
        let old_opacity = self.opacity;
        // 注意：这里需要内部可变性，后续用 RefCell 处理
        
        self.draw(ui, texture_handle, source_rect, position, color);
    }
    
    /// 设置全局透明度
    /// 
    /// C# equivalent: DXManager.SetOpacity(float)
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }
    
    /// 获取当前透明度
    /// 
    /// C# equivalent: DXManager.Opacity (property)
    pub fn opacity(&self) -> f32 {
        self.opacity
    }
    
    /// 设置灰度模式
    /// 
    /// C# equivalent: DXManager.SetGrayscale(bool)
    pub fn set_grayscale(&mut self, enabled: bool) {
        self.grayscale = enabled;
    }
    
    /// 设置混合模式
    /// 
    /// C# equivalent: DXManager.SetBlend(bool, float, BlendMode)
    pub fn set_blend(&mut self, enabled: bool, rate: f32) {
        self.blending = enabled;
        self.blending_rate = rate.clamp(0.0, 1.0);
    }
}
```

---

### Step 2: 增强 MLibrary（调用 DXManager）

**修改文件**: `ClientRust/src/graphics/texture_loader.rs`

```rust
impl MLibrary {
    /// 绘制 sprite
    /// 
    /// C# equivalent: MLibrary.Draw(int index, Point point, Color colour, bool offSet)
    /// 
    /// 注意：这个方法需要访问 DXManager 才能实际绘制
    pub fn draw(
        &mut self,
        dx_manager: &DXManager,
        ui: &mut egui::Ui,
        index: i32,
        point: (i32, i32),
        color: u32,
        use_offset: bool,
    ) -> io::Result<()> {
        // 1. 检查索引
        if !self.check_image(index) {
            return Ok(());
        }
        
        // 2. 加载纹理
        let (info, color_image) = self.load_color_image(index as usize)?;
        
        // 3. 计算位置（应用偏移）
        let mut x = point.0;
        let mut y = point.1;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        // 4. 屏幕裁剪检查
        // TODO: if (x >= ScreenWidth || y >= ScreenHeight) return;
        
        // 5. 创建或获取纹理句柄
        let texture_name = format!("{}_{}", self.path.display(), index);
        let texture_handle = dx_manager.ctx.load_texture(
            texture_name,
            color_image,
            Default::default()
        );
        
        // 6. 调用 DXManager.Draw
        dx_manager.draw(
            ui,
            &texture_handle,
            None, // 绘制整个纹理
            egui::pos2(x as f32, y as f32),
            color,
        );
        
        Ok(())
    }
    
    /// 绘制混合 sprite
    /// 
    /// C# equivalent: MLibrary.DrawBlend(int index, Point point, Color colour, bool offSet, float rate)
    pub fn draw_blend(
        &mut self,
        dx_manager: &DXManager,
        ui: &mut egui::Ui,
        index: i32,
        point: (i32, i32),
        color: u32,
        use_offset: bool,
        blend_rate: f32,
    ) -> io::Result<()> {
        // C# logic:
        // bool oldBlend = DXManager.Blending;
        // DXManager.SetBlend(true, rate);
        // DXManager.Draw(...);
        // DXManager.SetBlend(oldBlend);
        
        // TODO: 实现混合模式切换
        self.draw(dx_manager, ui, index, point, color, use_offset)
    }
}
```

---

### Step 3: PlayerObject 调用 MLibrary

**修改文件**: `ClientRust/src/objects/player_object.rs`

```rust
impl PlayerObject {
    /// 绘制角色
    /// 
    /// C# equivalent: PlayerObject.Draw() (line 4877)
    pub fn draw(
        &self,
        dx_manager: &DXManager,
        ui: &mut egui::Ui,
        libraries: &Libraries, // 新增：库管理器
        draw_location: Point,
    ) {
        // TODO: DrawBehindEffects
        
        // 处理透明度
        let old_opacity = dx_manager.opacity();
        if self.map_object.is_hidden() {
            dx_manager.set_opacity(0.5);
        }
        
        // 绘制坐骑
        self.draw_mount(dx_manager, ui, libraries, draw_location);
        
        // 绘制武器（Layer 1）
        // ...
        
        // 绘制身体
        self.draw_body(dx_manager, ui, libraries, draw_location);
        
        // 绘制头部
        self.draw_head(dx_manager, ui, libraries, draw_location);
        
        // 恢复透明度
        dx_manager.set_opacity(old_opacity);
    }
    
    /// 绘制身体
    /// 
    /// C# equivalent: PlayerObject.DrawBody() (line 5039)
    fn draw_body(
        &self,
        dx_manager: &DXManager,
        ui: &mut egui::Ui,
        libraries: &Libraries,
        draw_location: Point,
    ) {
        // C# code:
        // if (BodyLibrary != null)
        //     BodyLibrary.Draw(DrawFrame + ArmourOffSet, DrawLocation, drawColour, true);
        
        if let Some(body_lib) = libraries.get_body_library(self.class, self.armour) {
            let frame_index = self.calc_draw_frame(self.map_object.direction() as u8) 
                + self.armour_offset;
            let color = self.apply_draw_colour();
            
            let _ = body_lib.draw(
                dx_manager,
                ui,
                frame_index,
                (draw_location.x, draw_location.y),
                color,
                true, // use_offset
            );
        }
    }
}
```

---

## 📝 修正后的文件结构

```
ClientRust/
  src/
    graphics/
      mod.rs
      dx_manager.rs         ← 新增（对应 DXManager.cs）
      texture_loader.rs     ← 修改（MLibrary 增强）
      libraries.rs          ← 新增（Libraries 管理器）
    objects/
      player_object.rs      ← 修改（调用 MLibrary）
      ❌ renderer.rs        ← 删除（不存在的抽象）
```

---

## 🎯 下一步

1. ❌ 删除 `src/objects/renderer.rs`（已删除）
2. ✅ 创建 `src/graphics/dx_manager.rs`（正确移植）
3. ✅ 修改 `texture_loader.rs`（添加 draw 方法）
4. ✅ 创建 `libraries.rs`（库管理器）
5. ✅ 修改 `player_object.rs`（调用 MLibrary）

---

**教训**: 永远不要创造 C# 里不存在的抽象！严格移植！
