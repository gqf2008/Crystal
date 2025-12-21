use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;
use std::sync::OnceLock;

fn additive_material() -> &'static Material {
    static MATERIAL: OnceLock<Material> = OnceLock::new();
    MATERIAL.get_or_init(|| {
        load_material(
            ShaderSource::Glsl {
                vertex: include_str!("../../shaders/default.vert"),
                fragment: include_str!("../../shaders/default.frag"),
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    // C# DX: SourceAlpha + One
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .expect("failed to create additive material")
    })
}

/// 在加法混合 (SourceAlpha + One) 下执行绘制。
pub fn with_additive_blend(f: impl FnOnce()) {
    let material = additive_material();
    gl_use_material(material);
    f();
    gl_use_default_material();
}
