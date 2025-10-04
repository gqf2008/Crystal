// Sprite Shader - WGSL
// 2D sprite rendering with alpha blending

// 顶点着色器输入
struct VertexInput {
    @location(0) position: vec2<f32>,      // 单位正方形顶点 (-0.5 to 0.5)
    @location(1) tex_coords: vec2<f32>,    // 纹理坐标 (0 to 1)
}

// 实例数据输入
struct InstanceInput {
    @location(2) instance_position: vec2<f32>,  // 世界坐标 (像素)
    @location(3) instance_size: vec2<f32>,       // 精灵尺寸 (像素)
    @location(4) uv_offset: vec2<f32>,           // UV偏移
    @location(5) uv_scale: vec2<f32>,            // UV缩放
    @location(6) color: vec4<f32>,               // 颜色调制
}

// 顶点着色器输出 / 片段着色器输入
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

// 纹理和采样器
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// 顶点着色器
@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // 将单位正方形缩放到精灵尺寸
    let scaled_pos = vertex.position * instance.instance_size;
    
    // 平移到世界坐标
    let world_pos = scaled_pos + instance.instance_position;
    
    // TODO: 这里应该使用投影矩阵将世界坐标转换为NDC
    // 暂时假设屏幕坐标已经是NDC (-1 to 1)
    // 实际使用时需要传入屏幕尺寸并进行转换:
    // ndc_x = (world_x / screen_width) * 2.0 - 1.0
    // ndc_y = 1.0 - (world_y / screen_height) * 2.0
    
    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    
    // 应用UV变换
    out.tex_coords = vertex.tex_coords * instance.uv_scale + instance.uv_offset;
    
    // 传递颜色
    out.color = instance.color;
    
    return out;
}

// 片段着色器
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 采样纹理
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    // 颜色调制
    let final_color = tex_color * in.color;
    
    // 返回最终颜色 (包含alpha通道)
    return final_color;
}
