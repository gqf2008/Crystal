// ============================================================================
// 日夜循环（M11 简化版）
// 参考：C# TimeOfDay / macroquad day-night
// 简化：本地 24 分钟一天循环，夜晚叠加半透明深色覆盖层
// ============================================================================

use bevy::asset::Asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d};

use crate::scenes::AppState;
use crate::ui::sprite_ui::UiEntity;

/// 昼夜状态
#[derive(Resource)]
pub struct DayNight {
    /// 游戏时间（分钟，0..1440）
    pub time_minutes: f32,
    /// 一天实际时长（秒）
    pub day_length_secs: f32,
    /// 是否启用
    pub enabled: bool,
    /// 当前暗度 0..0.55（白天 0，夜晚最深；灯光按此显隐/渐显）
    pub darkness: f32,
}

impl Default for DayNight {
    fn default() -> Self {
        let start = std::env::var("BEVY_START_MINUTES")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(8.0 * 60.0);
        Self {
            time_minutes: start, // 默认早上 8 点；BEVY_START_MINUTES=1320 可切到晚上 22:00 验证灯光
            day_length_secs: 24.0 * 60.0, // 24 分钟一天
            enabled: true,
            darkness: 0.0,
        }
    }
}

/// 夜晚遮罩材质：标准 alpha 混合（ALPHA_BLENDING）的黑色半透明层。
/// 注意：Bevy 0.19 的 Material2d::specialize 自定义 blend 不生效（blend 由
/// alpha_mode() 决定），C# 的 darkness multiply（Zero/SourceColor）无法直接复刻，
/// 用黑色+alpha 近似：白天 color alpha=0（完全透明），夜晚 alpha=darkness 压暗。
/// 灯光在遮罩（z=0.8）之上（z=0.9），压暗后局部提亮。
#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct NightMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for NightMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/night.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component)]
pub struct NightOverlay;

pub struct DayNightPlugin;

impl Plugin for DayNightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DayNight>();
        app.add_plugins(Material2dPlugin::<NightMaterial>::default());
        app.add_systems(OnEnter(AppState::Game), spawn_overlay);
        app.add_systems(OnExit(AppState::Game), cleanup_overlay);
        app.add_systems(
            Update,
            day_night_system.run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_overlay(mut commands: Commands, overlay: Query<Entity, With<NightOverlay>>) {
    for e in overlay.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_overlay(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<NightMaterial>>,
) {
    let quad = meshes.add(Rectangle::new(1024.0, 768.0));
    let mat = materials.add(NightMaterial {
        color: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
    });
    commands.spawn((
        UiEntity,
        NightOverlay,
        Mesh2d(quad),
        MeshMaterial2d(mat),
        // z=0.8：只压暗场景（地图/角色 z<=0.5），灯光 0.9 在其上，UI z>=1 不受影响
        Transform::from_xyz(512.0, -384.0, 0.8),
        Visibility::Visible,
    ));
}

/// 推进时间并更新夜晚覆盖层透明度
fn day_night_system(
    mut dn: ResMut<DayNight>,
    time: Res<Time>,
    overlay: Query<&MeshMaterial2d<NightMaterial>>,
    mut materials: ResMut<Assets<NightMaterial>>,
    lights: Query<&MeshMaterial2d<crate::map_tile_anim::MapBlendMaterial>, With<crate::map_renderer::MapLight>>,
    mut blend_materials: ResMut<Assets<crate::map_tile_anim::MapBlendMaterial>>,
) {
    if !dn.enabled {
        return;
    }
    // 调试/体验：BEVY_FREEZE_TIME=1 冻结时间（保持在 BEVY_START_MINUTES），
    // 便于固定白天/夜晚验证灯光显隐。只跳过时间推进，灯光显隐仍要执行。
    if std::env::var("BEVY_FREEZE_TIME").is_err() {
        dn.time_minutes += time.delta_secs() * (1440.0 / dn.day_length_secs.max(1.0));
    }

    // 亮度曲线：6:00 日出 → 18:00 日落 → 24:00 最深
    let t = dn.time_minutes / 1440.0; // 0..1
    // 用正弦近似白天亮夜晚暗
    let daylight = ((t * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin() + 1.0) / 2.0;
    let darkness = (1.0 - daylight).clamp(0.0, 0.55);
    dn.darkness = darkness;

    // 遮罩：黑色 + darkness 透明度（白天 alpha=0 完全透明 → 无曝光）
    if let Ok(mat_handle) = overlay.single() {
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.color = LinearRgba::new(0.0, 0.0, 0.0, darkness);
        }
    }

    // 灯光显隐（对齐 C#：只有夜晚/黄昏才 DrawLights；白天不画灯光，避免过度曝光）
    // 注意：Bevy 0.19 的 Visibility::Hidden 对 Mesh2d 实体不生效（无 VisibilityClass），
    // 改用材质 alpha 控制——标准 alpha 混合下 alpha=0 时输出 dst，等效隐藏。
    // darkness 曲线：8:00 约 0.25、12:00 约 0、18:00 后 >0.4；
    // 阈值 0.4 = 只有明显夜晚/黄昏才亮灯（默认 8:00 清晨不亮）
    let light_on = darkness >= 0.4;
    let light_alpha = if light_on { 1.0 } else { 0.0 };
    for mat_handle in lights.iter() {
        if let Some(mut mat) = blend_materials.get_mut(&mat_handle.0) {
            // 强度 0.4 在 map_renderer 灯光生成时设定；这里只控制 alpha 显隐
            let arr = mat.color.to_f32_array();
            let base = arr[0].clamp(0.0, 1.0);
            mat.color = LinearRgba::new(base, base, base, light_alpha);
        }
    }
}
