// ============================================================================
// map_tile_anim.rs - 地图动画瓦片与灯光混合瓦片（参考 macroquad mesh_map_renderer）
// ============================================================================
// - Front 动画：animation = front_animation_frame；use_blend = animation & 0x80；
//   animation &= 0x7F；total = animation + animation*tick；
//   current_frame = (clock % total) / (1 + tick)；image = base + current_frame
// - blend 瓦片（0x80）：ADD 混合（Material2d specialize 自定义混合状态）
// - 定位：blend 顶边 = 格子底边 - 3 格；普通动画底边对齐格子底边
// Bevy 实现：动画帧 CPU 换图（Image 缓存）；blend 用 Mesh2d + MapBlendMaterial(ADD)

use bevy::asset::Asset;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
    RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::mesh::Mesh2d;
use bevy::sprite_render::{Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d};

use crate::map_renderer::{make_image, GameLibraries};
use crate::resources::libraries::Libraries;

/// 全局地图动画时钟（每帧 +1）
#[derive(Resource, Default)]
pub struct MapAnimClock {
    pub frame: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileAnimKind {
    Middle,
    Front,
}

#[derive(Component)]
pub struct MapTileAnim {
    pub kind: TileAnimKind,
    pub lib: i16,
    pub base_index: i32,
    pub frame_count: u8,
    pub tick: u8,
    pub blend: bool,
    pub left: f32,
    pub anchor_y: f32,
    pub top_anchored: bool,
    pub last_index: i32,
}

#[derive(Default, Resource)]
pub struct TileImageCache {
    pub map: std::collections::HashMap<(i16, i32), (Handle<Image>, u32, u32, i32, i32)>,
}

/// 地图灯光混合材质（ADD 混合，等价 macroquad add_blend_material）
#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct MapBlendMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

impl Material2d for MapBlendMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/map_blend.wgsl".into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let add = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut() {
                if let Some(t) = target {
                    t.blend = Some(add);
                }
            }
        }
        Ok(())
    }
}

/// 混合瓦片标记（Mesh2d + MapBlendMaterial）
#[derive(Component)]
pub struct BlendTile;

pub fn register_blend_material(app: &mut App) {
    app.add_plugins(Material2dPlugin::<MapBlendMaterial>::default());
}

/// 计算当前帧索引（macroquad 同款公式）
pub fn anim_index(kind: TileAnimKind, clock: u32, base: i32, frame_count: u8, tick: u8) -> i32 {
    let fc = frame_count.max(1) as u32;
    let t = tick as u32;
    let total = fc + fc * t;
    let cur = (clock % total) / (1 + t);
    base + cur as i32
}

/// 取瓦片帧图（带缓存）
pub fn tile_image(
    libs: &mut Libraries,
    images: &mut Assets<Image>,
    cache: &mut TileImageCache,
    lib: i16,
    idx: i32,
) -> Option<(Handle<Image>, u32, u32, i32, i32)> {
    if let Some(c) = cache.map.get(&(lib, idx)) {
        return Some(c.clone());
    }
    let info = libs.get_map_image(lib, idx)?;
    let rgba = info.rgba.clone()?;
    let (w, h) = (info.width.max(0) as u32, info.height.max(0) as u32);
    if w == 0 || h == 0 {
        return None;
    }
    let mut img = make_image(rgba, w, h);
    img.sampler = ImageSampler::nearest();
    let handle = images.add(img);
    let c = (handle, w, h, info.offset_x as i32, info.offset_y as i32);
    cache.map.insert((lib, idx), c.clone());
    Some(c)
}

/// 每帧推进动画时钟并更新动画瓦片帧图
pub fn map_tile_anim_system(
    mut clock: ResMut<MapAnimClock>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<TileImageCache>,
    mut sprites: Query<
        (&mut Sprite, &mut Transform, &mut MapTileAnim),
        (Without<BlendTile>, Without<MeshMaterial2d<MapBlendMaterial>>),
    >,
    mut blends: Query<
        (&MeshMaterial2d<MapBlendMaterial>, &mut Transform, &mut MapTileAnim),
        (With<BlendTile>, With<Mesh2d>),
    >,
    mut materials: ResMut<Assets<MapBlendMaterial>>,
) {
    clock.frame = clock.frame.wrapping_add(1);
    let f = clock.frame;
    for (mut sprite, mut tf, mut anim) in sprites.iter_mut() {
        let idx = anim_index(anim.kind, f, anim.base_index, anim.frame_count, anim.tick);
        if idx == anim.last_index {
            continue;
        }
        anim.last_index = idx;
        if let Some((handle, w, h, _, _)) =
            tile_image(&mut libs.0, &mut images, &mut cache, anim.lib, idx)
        {
            sprite.image = handle;
            let cy = if anim.top_anchored {
                anim.anchor_y - h as f32 / 2.0
            } else {
                anim.anchor_y + h as f32 / 2.0
            };
            tf.translation.x = anim.left + w as f32 / 2.0;
            tf.translation.y = cy;
        }
    }
    for (mut mat, mut tf, mut anim) in blends.iter_mut() {
        let idx = anim_index(anim.kind, f, anim.base_index, anim.frame_count, anim.tick);
        if idx == anim.last_index {
            continue;
        }
        anim.last_index = idx;
        if let Some((handle, w, h, _, _)) =
            tile_image(&mut libs.0, &mut images, &mut cache, anim.lib, idx)
        {
            if let Some(mut m) = materials.get_mut(mat.0.id()) {
                m.texture = handle;
            }
            let cy = if anim.top_anchored {
                anim.anchor_y - h as f32 / 2.0
            } else {
                anim.anchor_y + h as f32 / 2.0
            };
            tf.translation.x = anim.left + w as f32 / 2.0;
            tf.translation.y = cy;
            tf.scale = Vec3::new(w as f32, h as f32, 1.0);
        }
    }
}

/// 生成一个动画瓦片 Sprite（初始帧）
#[allow(clippy::too_many_arguments)]
pub fn spawn_anim_tile(
    commands: &mut Commands,
    libs: &mut Libraries,
    images: &mut Assets<Image>,
    cache: &mut TileImageCache,
    kind: TileAnimKind,
    lib: i16,
    base_index: i32,
    frame_count: u8,
    tick: u8,
    blend: bool,
    left: f32,
    anchor_y: f32,
    top_anchored: bool,
    z: f32,
) {
    if let Some((handle, w, h, _, _)) = tile_image(libs, images, cache, lib, base_index) {
        let cx = left + w as f32 / 2.0;
        let cy = if top_anchored {
            anchor_y - h as f32 / 2.0
        } else {
            anchor_y + h as f32 / 2.0
        };
        commands.spawn((
            Sprite::from_image(handle),
            Transform::from_xyz(cx, cy, z),
            Visibility::default(),
            MapTileAnim {
                kind, lib, base_index, frame_count, tick, blend,
                left, anchor_y, top_anchored, last_index: base_index,
            },
        ));
    }
}

/// 生成灯光混合瓦片（Mesh2d + MapBlendMaterial，ADD）
#[allow(clippy::too_many_arguments)]
pub fn spawn_blend_tile(
    commands: &mut Commands,
    libs: &mut Libraries,
    images: &mut Assets<Image>,
    cache: &mut TileImageCache,
    materials: &mut Assets<MapBlendMaterial>,
    quad: Handle<Mesh>,
    kind: TileAnimKind,
    lib: i16,
    base_index: i32,
    frame_count: u8,
    tick: u8,
    left: f32,
    anchor_y: f32,
    top_anchored: bool,
    z: f32,
) {
    if let Some((handle, w, h, _, _)) = tile_image(libs, images, cache, lib, base_index) {
        let cx = left + w as f32 / 2.0;
        let cy = if top_anchored {
            anchor_y - h as f32 / 2.0
        } else {
            anchor_y + h as f32 / 2.0
        };
        let mat = materials.add(MapBlendMaterial {
            color: LinearRgba::WHITE,
            texture: handle,
        });
        commands.spawn((
            BlendTile,
            Mesh2d(quad),
            MeshMaterial2d(mat),
            Transform::from_xyz(cx, cy, z).with_scale(Vec3::new(w as f32, h as f32, 1.0)),
            Visibility::default(),
            MapTileAnim {
                kind, lib, base_index, frame_count, tick, blend: true,
                left, anchor_y, top_anchored, last_index: base_index,
            },
        ));
    }
}
