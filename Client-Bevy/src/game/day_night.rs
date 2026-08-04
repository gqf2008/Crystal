// ============================================================================
// 日夜循环（M11 简化版）
// 参考：C# TimeOfDay / macroquad day-night
// 简化：本地 24 分钟一天循环，夜晚叠加半透明深色覆盖层
// ============================================================================

use bevy::asset::Asset;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
    RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d};

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
        }
    }
}

/// 夜晚遮罩材质：multiply 混合（dst = src * dst），对齐 C# DrawLights 把
/// darkness surface 用 Zero/SourceColor 乘回主画面的机制：
/// - 白天 color=1 → 不变；夜晚 color<1 → 按比例压暗（保留颜色，不会"洗白"）
/// - 灯光（ADD，z=0.9）在遮罩（z=0.8）之上，压暗后局部提亮，效果与 C# 一致
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

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // multiply：result.rgb = src.rgb * dst.rgb（C# SourceBlend=Zero, DestBlend=SourceColor）
        let multiply = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Dst,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut() {
                if let Some(t) = target {
                    t.blend = Some(multiply);
                }
            }
        }
        Ok(())
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
) {
    if !dn.enabled {
        return;
    }
    dn.time_minutes += time.delta_secs() * (1440.0 / dn.day_length_secs.max(1.0));

    // 亮度曲线：6:00 日出 → 18:00 日落 → 24:00 最深
    let t = dn.time_minutes / 1440.0; // 0..1
    // 用正弦近似白天亮夜晚暗
    let daylight = ((t * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin() + 1.0) / 2.0;
    let darkness = (1.0 - daylight).clamp(0.0, 0.55);

    // multiply 因子：白天 1.0（不变）→ 夜晚 (1 - 0.55)（压暗）
    let factor = 1.0 - darkness;
    if let Ok(mat_handle) = overlay.single() {
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.color = LinearRgba::new(factor, factor, factor, 1.0);
        }
    }
}
