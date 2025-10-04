# CharacterRenderer 使用指南

## 🚀 快速开始

### 1. 基本用法

```rust
use crate::graphics::CharacterRenderer;
use mir2_shared::enums::{MirClass, MirGender};

// 创建渲染器
let mut renderer = CharacterRenderer::new();

// 加载 ChrSel.lib
renderer.load_chrsel_library("Data/ChrSel.lib")?;

// 加载角色精灵为 egui 图像
let (image_info, color_image) = renderer.load_character_color_image(
    MirClass::Warrior,
    MirGender::Male,
    0  // 第一帧
)?;

// 在 egui 中显示
let texture = ctx.load_texture("warrior_preview", color_image, Default::default());
ui.image(&texture);
```

---

## 📋 API 文档

### CharacterRenderer::new()

创建新的角色渲染器实例。

```rust
pub fn new() -> Self
```

**返回**: 未加载资源库的渲染器实例

---

### load_chrsel_library()

加载 ChrSel.lib 角色资源库。

```rust
pub fn load_chrsel_library<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>
```

**参数**:
- `path`: ChrSel.lib 文件路径 (相对或绝对)

**返回**: 
- `Ok(())` - 加载成功
- `Err(io::Error)` - 文件不存在或格式错误

**示例**:
```rust
// 相对路径
renderer.load_chrsel_library("Data/ChrSel.lib")?;

// 绝对路径
renderer.load_chrsel_library("C:/Game/Data/ChrSel.lib")?;
```

---

### get_character_sprite_index()

计算角色精灵在 ChrSel.lib 中的索引。

```rust
pub fn get_character_sprite_index(
    &self,
    class: MirClass,
    gender: MirGender,
    frame: usize,
) -> usize
```

**参数**:
- `class`: 职业 (Warrior, Wizard, Taoist, Assassin, Archer)
- `gender`: 性别 (Male, Female)
- `frame`: 动画帧 (0-9, 超出会自动取模)

**返回**: 精灵索引 (0-99)

**索引映射**:
```
Warrior  Male:    0-9    Female:  10-19
Wizard   Male:   20-29   Female:  30-39
Taoist   Male:   40-49   Female:  50-59
Assassin Male:   60-69   Female:  70-79
Archer   Male:   80-89   Female:  90-99
```

**示例**:
```rust
// 男性战士第一帧 → 索引 0
let index = renderer.get_character_sprite_index(
    MirClass::Warrior,
    MirGender::Male,
    0
);
assert_eq!(index, 0);

// 女性法师第5帧 → 索引 35
let index = renderer.get_character_sprite_index(
    MirClass::Wizard,
    MirGender::Female,
    5
);
assert_eq!(index, 35);

// 帧数超过10会自动取模
let index = renderer.get_character_sprite_index(
    MirClass::Warrior,
    MirGender::Male,
    15  // 15 % 10 = 5
);
assert_eq!(index, 5);
```

---

### load_character_sprite_data()

加载角色精灵的原始 RGBA 数据。

```rust
pub fn load_character_sprite_data(
    &mut self,
    class: MirClass,
    gender: MirGender,
    frame: usize,
) -> io::Result<(ImageInfo, Vec<u8>)>
```

**参数**:
- `class`: 职业
- `gender`: 性别
- `frame`: 动画帧

**返回**:
- `Ok((ImageInfo, Vec<u8>))` - 图像信息和RGBA字节数组
- `Err(io::Error)` - 加载失败 (库未加载或索引无效)

**ImageInfo 字段**:
```rust
pub struct ImageInfo {
    pub width: i32,
    pub height: i32,
    pub offset_x: i16,
    pub offset_y: i16,
}
```

**示例**:
```rust
let (info, rgba_data) = renderer.load_character_sprite_data(
    MirClass::Warrior,
    MirGender::Male,
    0
)?;

println!("尺寸: {}×{}", info.width, info.height);
println!("偏移: ({}, {})", info.offset_x, info.offset_y);
println!("数据大小: {} 字节", rgba_data.len());
// 数据大小应该等于 width * height * 4 (RGBA)
```

---

### load_character_color_image()

加载角色精灵为 egui::ColorImage (推荐用于UI显示)。

```rust
pub fn load_character_color_image(
    &mut self,
    class: MirClass,
    gender: MirGender,
    frame: usize,
) -> io::Result<(ImageInfo, egui::ColorImage)>
```

**参数**:
- `class`: 职业
- `gender`: 性别
- `frame`: 动画帧

**返回**:
- `Ok((ImageInfo, ColorImage))` - 图像信息和egui颜色图像
- `Err(io::Error)` - 加载失败

**示例**:
```rust
let (info, color_image) = renderer.load_character_color_image(
    MirClass::Warrior,
    MirGender::Male,
    0
)?;

// 创建 egui 纹理
let texture = ui.ctx().load_texture(
    "warrior_preview",
    color_image,
    egui::TextureOptions::default(),
);

// 显示图像
ui.image(&texture);

// 或者指定大小
ui.image(&texture)
    .fit_to_exact_size(egui::vec2(100.0, 100.0));
```

---

### get_character_sprite_info()

仅获取精灵信息，不加载图像数据。

```rust
pub fn get_character_sprite_info(
    &mut self,
    class: MirClass,
    gender: MirGender,
    frame: usize,
) -> io::Result<ImageInfo>
```

**用途**: 预计算尺寸，优化布局

**示例**:
```rust
// 获取所有职业的精灵尺寸
for class in [MirClass::Warrior, MirClass::Wizard, ...] {
    let info = renderer.get_character_sprite_info(class, MirGender::Male, 0)?;
    println!("{:?}: {}×{}", class, info.width, info.height);
}
```

---

## 🎯 常见使用场景

### 场景 1: SelectScene 角色预览

```rust
pub struct SelectScene {
    character_preview_textures: HashMap<usize, egui::TextureHandle>,
}

impl SelectScene {
    fn render_character_preview(
        &mut self,
        ui: &mut egui::Ui,
        character_renderer: &mut CharacterRenderer,
        character: &SelectCharacter,
        slot_index: usize,
    ) {
        // 检查缓存
        if !self.character_preview_textures.contains_key(&slot_index) {
            // 加载角色精灵
            match character_renderer.load_character_color_image(
                character.class,
                character.gender,
                0
            ) {
                Ok((_, color_image)) => {
                    let texture = ui.ctx().load_texture(
                        format!("char_preview_{}", slot_index),
                        color_image,
                        egui::TextureOptions::default(),
                    );
                    self.character_preview_textures.insert(slot_index, texture);
                }
                Err(e) => {
                    tracing::warn!("加载角色精灵失败: {}", e);
                }
            }
        }
        
        // 显示图像
        if let Some(texture) = self.character_preview_textures.get(&slot_index) {
            ui.image(texture);
        }
    }
}
```

---

### 场景 2: 角色动画循环

```rust
pub struct CharacterAnimation {
    class: MirClass,
    gender: MirGender,
    current_frame: usize,
    last_update: Instant,
    texture: Option<egui::TextureHandle>,
}

impl CharacterAnimation {
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        renderer: &mut CharacterRenderer,
        delta_time: f32,
    ) {
        const FRAME_DURATION: f32 = 0.1; // 100ms per frame
        
        if self.last_update.elapsed().as_secs_f32() >= FRAME_DURATION {
            // 更新帧
            self.current_frame = (self.current_frame + 1) % 10;
            self.last_update = Instant::now();
            
            // 重新加载纹理
            if let Ok((_, color_image)) = renderer.load_character_color_image(
                self.class,
                self.gender,
                self.current_frame
            ) {
                self.texture = Some(ctx.load_texture(
                    format!("char_anim_{}_{}", self.class as u8, self.gender as u8),
                    color_image,
                    egui::TextureOptions::default(),
                ));
            }
        }
    }
    
    pub fn draw(&self, ui: &mut egui::Ui) {
        if let Some(ref texture) = self.texture {
            ui.image(texture);
        }
    }
}
```

---

### 场景 3: 角色创建预览

```rust
pub struct CharacterCreationDialog {
    preview_class: MirClass,
    preview_gender: MirGender,
    preview_frame: usize,
    preview_texture: Option<egui::TextureHandle>,
}

impl CharacterCreationDialog {
    fn render(
        &mut self,
        ui: &mut egui::Ui,
        character_renderer: &mut CharacterRenderer,
    ) {
        ui.horizontal(|ui| {
            // 职业选择
            if ui.button("战士").clicked() {
                self.preview_class = MirClass::Warrior;
                self.update_preview(ui.ctx(), character_renderer);
            }
            if ui.button("法师").clicked() {
                self.preview_class = MirClass::Wizard;
                self.update_preview(ui.ctx(), character_renderer);
            }
            // ...
        });
        
        ui.horizontal(|ui| {
            // 性别选择
            if ui.radio_value(&mut self.preview_gender, MirGender::Male, "男性").clicked() {
                self.update_preview(ui.ctx(), character_renderer);
            }
            if ui.radio_value(&mut self.preview_gender, MirGender::Female, "女性").clicked() {
                self.update_preview(ui.ctx(), character_renderer);
            }
        });
        
        // 显示预览
        if let Some(ref texture) = self.preview_texture {
            ui.image(texture)
                .fit_to_exact_size(egui::vec2(150.0, 150.0));
        }
    }
    
    fn update_preview(
        &mut self,
        ctx: &egui::Context,
        renderer: &mut CharacterRenderer,
    ) {
        if let Ok((_, color_image)) = renderer.load_character_color_image(
            self.preview_class,
            self.preview_gender,
            0
        ) {
            self.preview_texture = Some(ctx.load_texture(
                "creation_preview",
                color_image,
                egui::TextureOptions::default(),
            ));
        }
    }
}
```

---

## ⚠️ 错误处理

### 常见错误

#### 1. ChrSel.lib 未找到

```rust
match renderer.load_chrsel_library("Data/ChrSel.lib") {
    Ok(_) => {
        tracing::info!("ChrSel.lib 加载成功");
    }
    Err(e) => {
        tracing::error!("加载 ChrSel.lib 失败: {}", e);
        // 显示错误消息给用户
        ui.label("⚠️ 资源文件缺失，请检查 Data/ChrSel.lib");
    }
}
```

#### 2. 精灵索引无效

```rust
// 自动取模，不会出错
let index = renderer.get_character_sprite_index(
    MirClass::Warrior,
    MirGender::Male,
    999  // 999 % 10 = 9, 索引 9
);
```

#### 3. 加载图像失败

```rust
match renderer.load_character_color_image(class, gender, frame) {
    Ok((info, color_image)) => {
        // 成功
    }
    Err(e) => {
        tracing::warn!("加载精灵失败 (class={:?}, gender={:?}, frame={}): {}",
            class, gender, frame, e);
        // 显示占位图像
        ui.label("❌");
    }
}
```

---

## 🔧 性能优化建议

### 1. 纹理缓存

```rust
// ✅ 推荐：使用 HashMap 缓存
let mut texture_cache: HashMap<(MirClass, MirGender, usize), egui::TextureHandle> = HashMap::new();

fn get_or_load_texture(
    cache: &mut HashMap<(MirClass, MirGender, usize), egui::TextureHandle>,
    renderer: &mut CharacterRenderer,
    ctx: &egui::Context,
    class: MirClass,
    gender: MirGender,
    frame: usize,
) -> Option<egui::TextureHandle> {
    let key = (class, gender, frame);
    
    if !cache.contains_key(&key) {
        if let Ok((_, color_image)) = renderer.load_character_color_image(class, gender, frame) {
            let texture = ctx.load_texture(
                format!("char_{}_{}_{})", class as u8, gender as u8, frame),
                color_image,
                Default::default(),
            );
            cache.insert(key, texture);
        }
    }
    
    cache.get(&key).cloned()
}
```

### 2. 预加载

```rust
// 启动时预加载所有角色预览图像
fn preload_character_previews(
    renderer: &mut CharacterRenderer,
    ctx: &egui::Context,
) -> HashMap<(MirClass, MirGender), egui::TextureHandle> {
    let mut cache = HashMap::new();
    
    let classes = [
        MirClass::Warrior,
        MirClass::Wizard,
        MirClass::Taoist,
        MirClass::Assassin,
        MirClass::Archer,
    ];
    
    for class in &classes {
        for gender in &[MirGender::Male, MirGender::Female] {
            if let Ok((_, color_image)) = renderer.load_character_color_image(*class, *gender, 0) {
                let texture = ctx.load_texture(
                    format!("preload_{}_{}", *class as u8, *gender as u8),
                    color_image,
                    Default::default(),
                );
                cache.insert((*class, *gender), texture);
            }
        }
    }
    
    cache
}
```

### 3. 懒加载

```rust
// ❌ 避免：每帧重新加载
fn bad_example(ui: &mut egui::Ui, renderer: &mut CharacterRenderer) {
    // 这会导致性能问题！
    if let Ok((_, color_image)) = renderer.load_character_color_image(...) {
        let texture = ui.ctx().load_texture(...);
        ui.image(&texture);
    }
}

// ✅ 正确：仅在需要时加载一次
fn good_example(
    ui: &mut egui::Ui,
    renderer: &mut CharacterRenderer,
    cached_texture: &mut Option<egui::TextureHandle>,
) {
    if cached_texture.is_none() {
        if let Ok((_, color_image)) = renderer.load_character_color_image(...) {
            *cached_texture = Some(ui.ctx().load_texture(...));
        }
    }
    
    if let Some(ref texture) = cached_texture {
        ui.image(texture);
    }
}
```

---

## 📝 完整示例

### 完整的 SelectScene 集成

```rust
use std::collections::HashMap;
use crate::graphics::CharacterRenderer;
use mir2_shared::enums::{MirClass, MirGender};

pub struct SelectScene {
    characters: Vec<Option<SelectCharacter>>,
    selected_index: usize,
    character_preview_textures: HashMap<usize, egui::TextureHandle>,
}

pub struct SelectCharacter {
    pub index: u32,
    pub name: String,
    pub level: u16,
    pub class: MirClass,
    pub gender: MirGender,
}

impl SelectScene {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        character_renderer: &mut CharacterRenderer,
    ) {
        ui.vertical(|ui| {
            ui.heading("选择角色");
            
            for (idx, character_slot) in self.characters.iter().enumerate() {
                if let Some(character) = character_slot {
                    ui.horizontal(|ui| {
                        // 加载并显示角色预览
                        self.render_character_preview(
                            ui,
                            character_renderer,
                            character,
                            idx
                        );
                        
                        // 角色信息
                        ui.vertical(|ui| {
                            ui.label(format!("名称: {}", character.name));
                            ui.label(format!("等级: {}", character.level));
                            ui.label(format!("职业: {:?}", character.class));
                        });
                        
                        // 选择按钮
                        if ui.button("选择").clicked() {
                            self.selected_index = idx;
                        }
                    });
                } else {
                    // 空槽位
                    ui.horizontal(|ui| {
                        ui.label("空位");
                        if ui.button("创建角色").clicked() {
                            // 打开创建对话框
                        }
                    });
                }
                
                ui.separator();
            }
        });
    }
    
    fn render_character_preview(
        &mut self,
        ui: &mut egui::Ui,
        character_renderer: &mut CharacterRenderer,
        character: &SelectCharacter,
        slot_index: usize,
    ) {
        // 检查缓存
        if !self.character_preview_textures.contains_key(&slot_index) {
            // 加载精灵
            match character_renderer.load_character_color_image(
                character.class,
                character.gender,
                0
            ) {
                Ok((_, color_image)) => {
                    let texture = ui.ctx().load_texture(
                        format!("char_preview_{}", slot_index),
                        color_image,
                        egui::TextureOptions::default(),
                    );
                    self.character_preview_textures.insert(slot_index, texture);
                }
                Err(e) => {
                    tracing::warn!("加载角色精灵失败: {}", e);
                }
            }
        }
        
        // 显示图像
        if let Some(texture) = self.character_preview_textures.get(&slot_index) {
            ui.image(texture)
                .fit_to_exact_size(egui::vec2(80.0, 80.0));
        } else {
            // 加载失败，显示占位符
            ui.label("❌");
        }
    }
}
```

---

## 🎓 最佳实践

### ✅ DO (推荐)

1. **启动时加载 ChrSel.lib**
   ```rust
   let mut renderer = CharacterRenderer::new();
   renderer.load_chrsel_library("Data/ChrSel.lib")?;
   ```

2. **缓存纹理**
   ```rust
   let mut texture_cache: HashMap<...> = HashMap::new();
   ```

3. **错误处理**
   ```rust
   match renderer.load_character_color_image(...) {
       Ok(...) => { /* 使用 */ }
       Err(e) => { tracing::warn!("...", e); }
   }
   ```

4. **使用 load_character_color_image() 用于UI**
   ```rust
   let (_, color_image) = renderer.load_character_color_image(...)?;
   let texture = ctx.load_texture(...);
   ui.image(&texture);
   ```

### ❌ DON'T (避免)

1. **不要每帧重新加载**
   ```rust
   // ❌ 性能问题
   fn update(&mut self) {
       let (_, image) = self.renderer.load_character_color_image(...)?;
       self.texture = Some(ctx.load_texture(...));
   }
   ```

2. **不要忘记错误处理**
   ```rust
   // ❌ 可能崩溃
   let (_, image) = renderer.load_character_color_image(...).unwrap();
   ```

3. **不要直接使用精灵索引**
   ```rust
   // ❌ 容易出错
   let index = 23; // 这是什么？
   
   // ✅ 使用 API
   let index = renderer.get_character_sprite_index(MirClass::Wizard, MirGender::Male, 3);
   ```

---

## 📚 相关资源

- **实现报告**: `docs/p3-1-character-rendering-wgpu.md`
- **集成报告**: `docs/p3-1-integration-report.md`
- **单元测试**: `src/graphics/character_renderer.rs` (tests 模块)

---

**文档版本**: 1.0  
**最后更新**: 2025-10-04
