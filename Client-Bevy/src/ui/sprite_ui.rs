// ============================================================================
// sprite_ui - 用 Bevy Sprite 在精确屏幕坐标绘制 UI（对齐 macroquad draw_texture）
// ============================================================================

use std::collections::HashMap;

use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::map_renderer::{GameLibraries, make_image};
use crate::resources::libraries::LibraryName;

#[derive(Resource, Clone, Default)]
pub struct UiFont(pub Handle<Font>);

// ---------------------------------------------------------------------------
// UI 字体链（对齐 C# GDI：Settings.cs:72 FontName = "Arial"；中文经 GDI 字体链接
// 落到宋体）。主字体 = 系统 Arial（Latin/数字），Han 字形回退 = 系统宋体（setup_han_fallback_system）。
// 两者都从用户机器字体目录**运行时读取**（与 C# 走系统 GDI 同源，不打包进二进制，规避
// Arial/宋体的再分发许可）；系统字体缺失（非 Windows / 精简系统）回退内置 PuHuiTi 保底。
// ---------------------------------------------------------------------------
/// Windows 字体目录（%SystemRoot% 重定位的机器也正确；环境变量缺失回退 C:\Windows）
#[cfg(windows)]
fn system_fonts_dir() -> std::path::PathBuf {
    std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join("Fonts")
}

/// 选 UI 主字体字节：Windows 有系统 Arial 用之；否则回退内置 PuHuiTi。
/// 返回 (字节, 来源标签)。TTF magic 校验防字体目录里混入坏文件后整屏 tofu。
fn pick_ui_font_bytes() -> (Vec<u8>, &'static str) {
    #[cfg(windows)]
    {
        let path = system_fonts_dir().join("arial.ttf");
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() >= 4 && bytes[..4] == [0x00, 0x01, 0x00, 0x00] {
                return (bytes, "system-arial");
            }
        }
    }
    (
        include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf").to_vec(),
        "builtin-puhuiti",
    )
}

pub fn load_ui_font(assets: &mut Assets<Font>) -> Handle<Font> {
    let (bytes, kind) = pick_ui_font_bytes();
    tracing::info!("UI 主字体 = {kind}（C# Settings.FontName=Arial + GDI 链中文→宋体）");
    assets.add(Font::from_bytes(bytes))
}

/// 选 CJK 主字体字节（宋体资产，TTC）：动态改写文本的 UI 用——
/// 实机验证 parley 的 Hani 脚本回退只在实体首次排版时生效，后续重排版
/// （换页改文本）CJK 退化为 .notdef 豆腐（#2599）。主字体自带 CJK 则无需
/// 回退，规避该缺陷；宋体 = C# GDI 链中文最终命中的字体，视觉一致。
/// 缺失（非 Windows/精简系统）回退内置 PuHuiTi（自带 CJK）。
fn pick_cjk_font_bytes() -> (Vec<u8>, &'static str) {
    #[cfg(windows)]
    {
        let path = system_fonts_dir().join("simsun.ttc");
        if let Ok(bytes) = std::fs::read(&path) {
            // TTC magic 'ttcf'
            if bytes.len() >= 4 && &bytes[..4] == b"ttcf" {
                return (bytes, "system-simsun");
            }
        }
    }
    (
        include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf").to_vec(),
        "builtin-puhuiti",
    )
}

/// 加载 CJK 主字体资产（见 [pick_cjk_font_bytes]）
pub fn load_cjk_font(assets: &mut Assets<Font>) -> Handle<Font> {
    let (bytes, kind) = pick_cjk_font_bytes();
    tracing::info!("CJK 主字体 = {kind}（动态文本主字体，规避 #2599 重排版回退失效）");
    assets.add(Font::from_bytes(bytes))
}

/// 共享 CJK 主字体资源（#2602 批R）：多个对话框（NPC/公告/…）动态文本共用
/// 同一宋体资产——各自调 load_cjk_font 会重复 assets.add ~10MB 字节
#[derive(Resource, Clone, Default)]
pub struct UiCjkFont(pub Handle<Font>);

/// 惰性取共享 CJK 字体句柄（首个调用方加载，其余复用）
pub fn shared_cjk_font(assets: &mut Assets<Font>, res: &mut UiCjkFont) -> Handle<Font> {
    if !res.0.is_strong() {
        res.0 = load_cjk_font(assets);
    }
    res.0.clone()
}

/// 注册系统宋体并设为 Han 字形回退（复刻 GDI 字体链接：Arial 无中文 → 宋体）。
/// Startup 一次；SimSun 家族已在集合（重复注册）时仅确保 fallback 指向它。
/// 非 Windows / 无宋体：打 warn 后跳过（中文由主字体自带 CJK 或 tofu，与现状一致）。
pub fn setup_han_fallback_system(mut font_cx: ResMut<bevy::text::FontCx>) {
    #[cfg(windows)]
    {
        use parley::fontique::{FallbackKey, Script};

        let han = FallbackKey::new(Script::from_bytes(*b"Hani"), None);
        // 集合里已有 SimSun 家族（如重复调用）直接复用，避免重复注册字体数据
        let existing = font_cx.collection.family_id("SimSun");
        let target = match existing {
            Some(id) => id,
            None => {
                let path = system_fonts_dir().join("simsun.ttc");
                let Ok(bytes) = std::fs::read(&path) else {
                    tracing::warn!("未找到系统宋体（{}），中文回退沿用默认链", path.display());
                    return;
                };
                let registered = font_cx
                    .collection
                    .register_fonts(parley::fontique::Blob::from(bytes), None);
                // simsun.ttc 面序：face0=SimSun、face1=NSimSun；按名精确挑 SimSun
                let picked = registered.iter().find_map(|(fid, _)| {
                    font_cx
                        .collection
                        .family_name(*fid)
                        .filter(|n| n.eq_ignore_ascii_case("SimSun"))
                        .map(|_| *fid)
                });
                match picked.or_else(|| registered.first().map(|(fid, _)| *fid)) {
                    Some(id) => id,
                    None => {
                        tracing::warn!("系统宋体注册失败（simsun.ttc 解析为空）");
                        return;
                    }
                }
            }
        };
        font_cx
            .collection
            .set_fallbacks(han, std::iter::once(target));
        tracing::info!("Han 字形回退 = 系统 SimSun（对齐 C# GDI 字体链接）");
    }
    #[cfg(not(windows))]
    {
        let _ = &mut font_cx;
    }
}

#[cfg(test)]
mod font_tests {
    use super::*;

    /// 主字体选取：Windows 有系统 Arial 时必须选中它。
    /// C# Settings.cs:72 FontName="Arial" —— 本测试锚定「用系统 Arial」这一行为本身：
    /// 镜像 fs 读取 + magic 判定，文件有效时**强制**断言 kind=="system-arial"（若实现
    /// 回归为永不选系统字体，本测试必须失败，而非静默走回退分支）。
    #[test]
    fn ui_font_prefers_system_arial() {
        let (bytes, kind) = pick_ui_font_bytes();
        #[cfg(windows)]
        {
            // 镜像实现的前置条件：字体目录里有合法 arial.ttf（TTF magic 头）
            let file_valid = std::fs::read(system_fonts_dir().join("arial.ttf"))
                .map(|b| b.len() >= 4 && b[..4] == [0x00, 0x01, 0x00, 0x00])
                .unwrap_or(false);
            if file_valid {
                assert_eq!(kind, "system-arial", "有合法系统 Arial 却未选中");
                assert!(bytes.len() > 4, "Arial 字节非空");
                assert_eq!(&bytes[..4], &[0x00, 0x01, 0x00, 0x00], "TTF magic");
                let builtin: &[u8] =
                    include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
                assert_ne!(
                    bytes.as_slice(),
                    builtin,
                    "选中的应是系统 Arial 而非内置字体"
                );
                return;
            }
        }
        // 无系统 Arial 的环境（非 Windows / 字体目录被清）：回退内置 PuHuiTi保底
        assert_eq!(kind, "builtin-puhuiti");
        assert!(!bytes.is_empty());
    }

    /// Han 回退注册行为：setup 后 fontique 集合存在 SimSun 家族，且 Script(Hani)
    /// 的 fallback 列表指向它（Windows 有 simsun.ttc 时；缺失环境跳过断言）。
    #[test]
    #[cfg(windows)]
    fn han_fallback_registers_simsun() {
        use parley::fontique::{FallbackKey, Script};

        let mut app = bevy::app::App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
        ));
        app.add_systems(Startup, setup_han_fallback_system);
        app.update();

        let world = app.world_mut();
        let mut font_cx = world
            .get_resource_mut::<bevy::text::FontCx>()
            .expect("TextPlugin 应初始化 FontCx");
        if !system_fonts_dir().join("simsun.ttc").exists() {
            return; // 精简 Windows 无宋体：行为=打 warn 跳过，不断言
        }
        let simsun = font_cx
            .collection
            .family_id("SimSun")
            .expect("注册后集合应含 SimSun 家族");
        let han = FallbackKey::new(Script::from_bytes(*b"Hani"), None);
        let fallbacks: Vec<_> = font_cx.collection.fallback_families(han).collect();
        assert!(
            fallbacks.contains(&simsun),
            "Script(Hani) 回退应包含 SimSun（实际 {:?}）",
            fallbacks
        );
    }
}

/// 调试用 UI 子系统开关（UI_BITS 环境变量，逗号分隔，如 "hud,chat"）。
/// 未设置时全部显示；设置后只显示列出的子系统。
/// 子系统名：hud / chat / map(小地图) / skill(技能栏) / quest(任务追踪)
pub fn ui_enabled(subsystem: &str) -> bool {
    match std::env::var("UI_BITS") {
        Ok(v) => v.split(',').any(|s| s.trim() == subsystem),
        Err(_) => true,
    }
}

/// UI 图像缓存
#[derive(Resource, Default)]
pub struct UiImageCache {
    pub map: HashMap<(u8, usize), Handle<Image>>,
}

/// 标记所有 UI 精灵/文本/按钮，退出场景时统一清理
#[derive(Component)]
pub struct UiEntity;

/// 给所有 UI 实体加渲染层 1（只被 UI 相机渲染，避免 UI 相机重画地图）
pub fn mark_ui_render_layers(q: Query<Entity, Added<UiEntity>>, mut commands: Commands) {
    for e in &q {
        commands
            .entity(e)
            .try_insert(bevy::camera::visibility::RenderLayers::layer(1));
    }
}

/// 向下传播 UI 渲染层（#2521）：RenderLayers 不随层级传播（Bevy 0.19
/// check_visibility_cpu_culling 无父级回溯），挂在 UiEntity 父实体下、自己没挂
/// UiEntity 的 Sprite/Text2d 子/孙控件默认 layer 0，会被 UI 相机剔除 → 不可见。
/// 本系统对「UiEntity 实体的后代」统一补 layer 1（try_insert 不覆盖显式层，
/// 如 day_night 的 layer 2）。触发：Added<UiEntity>（新 UI 根）+
/// Added/Changed<ChildOf>（新/重挂子实体）。与 mark_ui_render_layers 并列注册：
/// 实体自身由原系统处理，本系统只管子/孙层级。
pub fn propagate_ui_render_layers(
    triggers: Query<Entity, Or<(Added<UiEntity>, Added<ChildOf>, Changed<ChildOf>)>>,
    ui_marked: Query<(), With<UiEntity>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    has_layers: Query<(), With<RenderLayers>>,
    mut commands: Commands,
) {
    for e in &triggers {
        // 沿父链上溯：本实体或任一祖先挂 UiEntity 才算 UI 子树（世界实体父链无 UiEntity，不触及）
        let mut under_ui = false;
        let mut cur = e;
        for _ in 0..64 {
            if ui_marked.contains(cur) {
                under_ui = true;
                break;
            }
            let Ok(parent) = parents.get(cur) else {
                break;
            };
            cur = parent.0;
        }
        if !under_ui {
            continue;
        }
        // 向下 DFS 整棵子树补 layer 1（已有显式 RenderLayers 的不覆盖）
        let mut stack = vec![e];
        while let Some(n) = stack.pop() {
            if !has_layers.contains(n) {
                commands.entity(n).try_insert(RenderLayers::layer(1));
            }
            if let Ok(cs) = children.get(n) {
                for c in &*cs {
                    stack.push(*c);
                }
            }
        }
    }
}

/// UI 相机（世界坐标 = 屏幕逻辑像素 0..1024 x 0..768，y 向下）
/// 带 UiEntity 标记，随场景退出清理，避免泄漏到游戏场景
pub fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        UiEntity,
        Camera2d,
        Transform::from_xyz(512.0, -384.0, 100.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 1024.0,
                height: 768.0,
            },
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: 1,
            ..default()
        },
        // 只渲染 UI 层，避免把地图实体也画一遍（见 game/mod.rs）
        bevy::camera::visibility::RenderLayers::from_layers(&[1, 2]),
    ));
}

/// 按库+索引加载图像（缓存），返回句柄
pub fn ui_image(
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    index: usize,
) -> Option<Handle<Image>> {
    let key = (name as u8, index);
    if let Some(h) = cache.map.get(&key) {
        return Some(h.clone());
    }
    let info = libs.0.get_image(name, index)?;
    let rgba = info.rgba.clone()?;
    let w = info.width.max(0) as u32;
    let h = info.height.max(0) as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let handle = images.add(make_image(rgba, w, h));
    cache.map.insert(key, handle.clone());
    Some(handle)
}

/// 生成 UI 精灵（屏幕坐标 x,y 左上角，y 向下；scale 缩放）
pub fn spawn_ui_sprite(
    commands: &mut Commands,
    handle: Handle<Image>,
    x: f32,
    y: f32,
    z: f32,
    scale: f32,
) -> Entity {
    commands
        .spawn((
            UiEntity,
            Sprite::from_image(handle),
            // 屏幕坐标 = 纹理左上角（原版 draw_texture 约定）
            Anchor::TOP_LEFT,
            Transform::from_xyz(x, -y, z).with_scale(Vec3::splat(scale)),
            Visibility::default(),
        ))
        .id()
}

/// UI 文本（Text2d，屏幕坐标，y 向下）
pub fn spawn_ui_text(
    commands: &mut Commands,
    font: &Handle<Font>,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    z: f32,
) -> Entity {
    commands
        .spawn((
            UiEntity,
            Text2d::new(text),
            // 左上角锚定，与宏模块 draw_text 左上角坐标一致
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id()
}

/// UI 居中文本（Text2d）：`anchor` 为锚点、`(x,y)` 传锚点（屏幕坐标，y 向下），内容变化自动重排。
/// 复刻 C# MirLabel 居中语义：`DrawFormat=HCenter|VCenter` 用 `Anchor::CENTER` 锚定框心；
/// `Label_SizeChanged{x=中心-width/2}` 用 `Anchor::TOP_CENTER` 锚定顶边中点。
#[allow(clippy::too_many_arguments)]
pub fn spawn_ui_text_anchored(
    commands: &mut Commands,
    font: &Handle<Font>,
    text: &str,
    anchor: Anchor,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    z: f32,
) -> Entity {
    commands
        .spawn((
            UiEntity,
            Text2d::new(text),
            anchor,
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id()
}

/// UI 按钮（矩形命中测试）
#[derive(Component)]
pub struct UiButton {
    pub rect: (f32, f32, f32, f32),
    pub clicked: bool,
}

/// 三态按钮帧（normal/hover/pressed），与原版 draw_button 一致
#[derive(Component)]
pub struct ButtonFrames {
    pub normal: Handle<Image>,
    pub hover: Handle<Image>,
    pub pressed: Handle<Image>,
}

/// 生成带三态帧的按钮（normal_idx/hover_idx/pressed_idx）
pub fn spawn_ui_button(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    name: LibraryName,
    normal_idx: usize,
    hover_idx: usize,
    pressed_idx: usize,
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    h: f32,
) -> Option<Entity> {
    let normal = ui_image(libs, images, cache, name, normal_idx)?;
    let hover = ui_image(libs, images, cache, name, hover_idx)?;
    let pressed = ui_image(libs, images, cache, name, pressed_idx)?;
    let e = spawn_ui_sprite(commands, normal.clone(), x, y, z, 1.0);
    commands.entity(e).insert((
        UiButton {
            rect: (x, y, w, h),
            clicked: false,
        },
        ButtonFrames {
            normal,
            hover,
            pressed,
        },
    ));
    Some(e)
}

/// 按钮系统：鼠标左键按下时命中矩形 → clicked=true；
/// 带 ButtonFrames 的按钮按 hover/pressed 状态切换帧
pub fn ui_button_system(
    mut buttons: Query<(&mut UiButton, Option<&mut ButtonFrames>, &mut Sprite)>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    // UI 相机 Fixed 1024x768：窗口缩放/最大化时需换算成 UI 世界坐标（按钮命中保持准确）
    let Ok((cam, gtf)) = ui_cameras.single() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else {
        return;
    };
    let cursor = Vec2::new(world.x, -world.y);
    let just = mouse.just_pressed(MouseButton::Left);
    let down = mouse.pressed(MouseButton::Left);
    // #幽灵鼠标：窗口失焦/后台时 macOS 可能残留 ButtonInput<MouseButton> 为 just_pressed，
    // 重聚焦后把残留点击当作新点击（症状：进游戏后 HUD 按钮被“自动点击”，技能窗口自己开关）。
    // 加窗口聚焦门控，失焦时不响应任何按钮点击（对齐 #2618 键盘聚焦门控）。
    let focused = window.focused;
    for (mut btn, frames, mut sprite) in &mut buttons {
        let (x, y, w, h) = btn.rect;
        let over = cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h;
        btn.clicked = just && over && focused;
        if let Some(frames) = frames {
            let frame = if down && over {
                &frames.pressed
            } else if over {
                &frames.hover
            } else {
                &frames.normal
            };
            if sprite.image != *frame {
                sprite.image = frame.clone();
            }
        }
    }
}

/// 按钮点击音效覆盖（#91）：默认 ButtonB=10104，可指定 C# SoundList 音效 id
#[derive(Component)]
pub struct ButtonSound(pub u32);

/// 按钮悬停进入音效（#91，可选挂载；C# MirButton 默认只有点击音效）
#[derive(Component)]
pub struct ButtonHoverSound(pub u32);

/// UI 按钮音效系统（#91）：
/// - 点击：播放 ButtonSound 覆盖或默认 ButtonB(10104)（对齐 C# MirControl.OnMouseClick）
/// - 悬停进入：仅对挂了 ButtonHoverSound 的按钮播放一次
pub fn ui_button_sound_system(
    mut commands: Commands,
    mut assets: ResMut<Assets<bevy::audio::AudioSource>>,
    bank: Res<crate::game::sound::SoundBank>,
    mut cache: ResMut<crate::game::sound::SoundCache>,
    windows: Query<&Window>,
    buttons: Query<(
        Entity,
        &UiButton,
        Option<&ButtonSound>,
        Option<&ButtonHoverSound>,
    )>,
    mut hovered_prev: Local<std::collections::HashSet<Entity>>,
    ui_cameras: Query<(&Camera, &GlobalTransform), With<UiEntity>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((cam, gtf)) = ui_cameras.single() else {
        return;
    };
    let Ok(world) = cam.viewport_to_world_2d(gtf, cursor) else {
        return;
    };
    let cursor = Vec2::new(world.x, -world.y);

    let mut hovered_now = std::collections::HashSet::new();
    for (e, btn, sound, hover_sound) in &buttons {
        let (x, y, w, h) = btn.rect;
        let over = cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h;
        if over {
            hovered_now.insert(e);
        }
        // 点击音效
        if btn.clicked {
            let id = sound.map(|s| s.0).unwrap_or(10104); // C# SoundList.ButtonB
            crate::game::sound::play_sound_cached(
                &mut commands,
                &mut assets,
                &bank,
                &mut cache,
                id,
            );
        }
        // 悬停进入音效（可选）
        if over && !hovered_prev.contains(&e) {
            if let Some(hs) = hover_sound {
                crate::game::sound::play_sound_cached(
                    &mut commands,
                    &mut assets,
                    &bank,
                    &mut cache,
                    hs.0,
                );
            }
        }
    }
    *hovered_prev = hovered_now;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    /// #2521 回归：UiEntity 根下 with_children 裸 spawn 的子/孙控件都应有 layer 1
    #[test]
    fn ui_subtree_children_get_layer_1() {
        let mut app = App::new();
        let mut child = Entity::PLACEHOLDER;
        let mut grandchild = Entity::PLACEHOLDER;
        let root = app
            .world_mut()
            .spawn(UiEntity)
            .with_children(|p| {
                child = p
                    .spawn(())
                    .with_children(|g| {
                        grandchild = g.spawn(()).id();
                    })
                    .id();
            })
            .id();
        app.world_mut()
            .run_system_once(propagate_ui_render_layers)
            .expect("propagate_ui_render_layers 应可运行");

        let mut layers = app.world_mut().query::<&RenderLayers>();
        for e in [root, child, grandchild] {
            assert_eq!(
                *layers.get(app.world(), e).unwrap_or_else(|err| {
                    panic!("实体 {e:?} 应被补上 layer 1：{err}");
                }),
                RenderLayers::layer(1),
                "实体 {e:?} 应有 layer 1"
            );
        }
    }

    /// #2521 回归：已有显式 RenderLayers 的子控件不被传播覆盖（如 day_night 的 layer 2）
    #[test]
    fn explicit_layers_not_overridden() {
        let mut app = App::new();
        let mut child = Entity::PLACEHOLDER;
        let _root = app
            .world_mut()
            .spawn(UiEntity)
            .with_children(|p| {
                child = p.spawn(RenderLayers::layer(2)).id();
            })
            .id();
        app.world_mut()
            .run_system_once(propagate_ui_render_layers)
            .expect("propagate_ui_render_layers 应可运行");

        let mut layers = app.world_mut().query::<&RenderLayers>();
        assert_eq!(
            *layers.get(app.world(), child).unwrap(),
            RenderLayers::layer(2),
            "显式 layer 2 不应被传播覆盖"
        );
    }

    /// #2521 回归：父链无 UiEntity 的实体（世界空间 actor 子树等）不误挂 layer 1
    #[test]
    fn non_ui_subtree_untouched() {
        let mut app = App::new();
        let mut child = Entity::PLACEHOLDER;
        let root = app
            .world_mut()
            .spawn(())
            .with_children(|p| {
                child = p.spawn(()).id();
            })
            .id();
        app.world_mut()
            .run_system_once(propagate_ui_render_layers)
            .expect("propagate_ui_render_layers 应可运行");

        let mut layers = app.world_mut().query::<&RenderLayers>();
        assert!(
            layers.get(app.world(), root).is_err(),
            "非 UI 根不应被挂 RenderLayers"
        );
        assert!(
            layers.get(app.world(), child).is_err(),
            "非 UI 子树子实体不应被挂 RenderLayers"
        );
    }
}
