//! 对象特效的**加法混合**材质。
//!
//! 原版语义（2026-09-25 按 C# 源码钉死，别再靠猜）：
//! - `Effect.Blend`（`Client/MirObjects/Effect.cs:23`，默认 `true`）为真时走
//!   `Library.DrawBlend(...)` → `MLibrary.cs:772` `DXManager.SetBlend(true, rate)` →
//!   `DXManager.cs:378-379` **SourceBlend=SourceAlpha / DestinationBlend=One = 加法混合**；
//! - 为假时走 `Library.Draw(...)`（`Effect.cs:132/214`）→ 当时批次为
//!   `SpriteFlags.AlphaBlend` → **普通 alpha over**。
//!
//! 本端的偏差：`GameScene.ObjectEffect` 这 39 条里 **31 条是 `Blend=true`**（只有
//! RedMoonEvil / 觉醒 4 条 / Behemoth / KingGuard×2 共 8 条显式 `blend: false`），
//! 而旧实现对所有条目都 spawn 普通 `Sprite`（Bevy 的 Sprite 只有 alpha over）——
//! 也就是说**发光类的护盾/治疗/传送/天罚…一直被画成了普通贴图**，`blend=false` 那 8 条反而是对的。
//!
//! 所以这里只给 `blend = true` 的条目提供加法材质；`blend = false` 继续用普通 `Sprite`
//! （= 原版 `Library.Draw` 的语义），改动面最小、不碰已经对的那条路径。
//!
//! 着色器直接复用 `shaders/map_blend.wgsl`（采样 × tint，绑定与 `MapBlendMaterial` 完全一致：
//! uniform(0)=color、texture(1)、sampler(2)）。

use bevy::asset::Asset;
use bevy::mesh::Mesh2d;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{
    AlphaMode2d, Material2d, Material2dKey, Material2dPlugin, MeshMaterial2d,
};

/// 加法混合材质（原版 `Blend = true` 的那 31 条 ObjectEffect 用）
#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
pub struct ObjectFxBlendMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

impl Material2d for ObjectFxBlendMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/map_blend.wgsl".into()
    }

    /// Material2d 默认 `alpha_mode = Opaque` 会走不透明管线（blend=None），
    /// 特效美术按 ADD 设计、黑底会被画成不透明黑块 —— 必须声明 Blend 再在 specialize 里改成 ADD
    /// （与 `MapBlendMaterial` 同一处置，实机踩过）。
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let add = additive_blend_state();
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

/// 与 C# `DXManager.SetBlend(true, rate)` 一字不差：`SourceBlend=SourceAlpha` / `DestinationBlend=One`。
/// 抽成纯函数是为了让它能被单测直接钉住（构造 RenderPipelineDescriptor 做断言会绑死 Bevy 内部结构）。
pub(crate) fn additive_blend_state() -> BlendState {
    let component = BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    };
    BlendState {
        color: component,
        alpha: component,
    }
}

/// 1×1 单位四边形：Mesh2d 没有 `custom_size`，尺寸靠 `Transform::scale`（像素）表达。
/// 惰性初始化（`Assets<Mesh>` 未必在插件 build 阶段就绪），缓存在资源里避免每次 spawn 都新建。
#[derive(Resource, Default)]
pub struct ObjectFxQuad(pub Option<Handle<Mesh>>);

pub fn register_object_fx_material(app: &mut App) {
    app.add_plugins(Material2dPlugin::<ObjectFxBlendMaterial>::default());
    app.init_resource::<ObjectFxQuad>();
}

/// 取（必要时创建）单位四边形句柄
pub fn object_fx_quad(meshes: &mut Assets<Mesh>, cache: &mut ObjectFxQuad) -> Handle<Mesh> {
    cache
        .0
        .get_or_insert_with(|| meshes.add(Rectangle::new(1.0, 1.0)))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 门禁：加法材质的混合状态必须是 SrcAlpha/One（= C# `DXManager.SetBlend(true)`，
    /// `Client/MirGraphics/DXManager.cs:378-379`）。
    /// **阳性对照**：把 dst_factor 改成 `InverseSrcAlpha`（= 普通 alpha）→ 本测试立即红。
    #[test]
    fn additive_blend_state_matches_csharp_set_blend_true() {
        let b = additive_blend_state();
        assert_eq!(
            b.color.src_factor,
            BlendFactor::SrcAlpha,
            "源因子应为 SrcAlpha"
        );
        assert_eq!(
            b.color.dst_factor,
            BlendFactor::One,
            "ADD 的目标因子必须是 One（普通 alpha 是 InverseSrcAlpha）"
        );
        assert_eq!(b.color.operation, BlendOperation::Add);
        assert_ne!(
            b.color.dst_factor,
            BlendFactor::OneMinusSrcAlpha,
            "不能退化成普通 alpha over"
        );
    }
}
