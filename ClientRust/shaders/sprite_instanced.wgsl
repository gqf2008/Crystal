// Sprite Instanced Shader - GPU实例化渲染
// 用于批量渲染大量相同纹理的精灵(如粒子系统)

// 顶点Uniforms (屏幕空间变换)
struct VertexUniforms {
    screen_size: vec2<f32>,
}

// 片段Uniforms (颜色调制等)
struct FragmentUniforms {
    color: vec4<f32>,        // 全局颜色调制 (通常为白色)
    opacity: f32,            // 全局透明度
    grayscale: f32,          // 灰度模式
}

// 顶点输入 (quad模板,所有实例共享)
struct VertexInput {
    @location(0) position: vec2<f32>,     // 局部坐标 (0,0 到 1,1)
    @location(1) tex_coords: vec2<f32>,   // 纹理坐标
}

// 实例输入 (每个粒子独立的数据)
struct InstanceInput {
    @location(2) instance_position: vec2<f32>,  // 世界坐标
    @location(3) instance_size: vec2<f32>,      // 精灵尺寸
    @location(4) instance_color: vec4<f32>,     // 精灵颜色
}

// 顶点输出
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> vertex_uniforms: VertexUniforms;
@group(1) @binding(0) var texture_sampler: sampler;
@group(1) @binding(1) var texture: texture_2d<f32>;
@group(2) @binding(0) var<uniform> fragment_uniforms: FragmentUniforms;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // 将局部坐标(0,0到1,1)转换为实际像素坐标
    let world_pos = instance.instance_position + vertex.position * instance.instance_size;
    
    // 屏幕空间归一化 (0,0到width,height → -1,-1到1,1)
    let normalized_x = (world_pos.x / vertex_uniforms.screen_size.x) * 2.0 - 1.0;
    let normalized_y = 1.0 - (world_pos.y / vertex_uniforms.screen_size.y) * 2.0;
    
    out.clip_position = vec4<f32>(normalized_x, normalized_y, 0.0, 1.0);
    out.tex_coords = vertex.tex_coords;
    out.color = instance.instance_color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 采样纹理
    var texture_color = textureSample(texture, texture_sampler, in.tex_coords);
    
    // 应用实例颜色调制
    texture_color = texture_color * in.color;
    
    // 应用全局颜色调制
    texture_color = texture_color * fragment_uniforms.color;
    
    // 应用全局透明度
    texture_color.a = texture_color.a * fragment_uniforms.opacity;
    
    // 灰度模式
    if (fragment_uniforms.grayscale > 0.5) {
        let gray = dot(texture_color.rgb, vec3<f32>(0.299, 0.587, 0.114));
        texture_color = vec4<f32>(gray, gray, gray, texture_color.a);
    }
    
    return texture_color;
}
