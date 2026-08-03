#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
}

struct MapBlendMaterial {
    color: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: MapBlendMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var map_blend_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var map_blend_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(map_blend_texture, map_blend_sampler, mesh.uv);
    return tex * material.color;
}
