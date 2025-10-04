// Sprite Shader - 2D 精灵渲染
// 对应 C# 的 SlimDX.Sprite 渲染功能
//
// C# 使用 DirectX 9 固定管线，这里使用可编程管线复刻相同效果

// ===== 顶点着色器 =====

struct VertexInput {
    @location(0) position: vec2<f32>,     // 屏幕空间位置 (x, y)
    @location(1) tex_coords: vec2<f32>,   // 纹理坐标 (u, v)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // 裁剪空间位置
    @location(0) tex_coords: vec2<f32>,           // 传递给片段着色器的纹理坐标
}

struct Uniforms {
    screen_size: vec2<f32>,    // 屏幕尺寸 (width, height)
    _padding: vec2<f32>,       // 对齐到 16 字节
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // 将屏幕坐标转换为裁剪空间坐标
    // 屏幕坐标: (0, 0) 在左上角，(width, height) 在右下角
    // 裁剪空间: (-1, -1) 在左下角，(1, 1) 在右上角
    let normalized_x = (input.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let normalized_y = 1.0 - (input.position.y / uniforms.screen_size.y) * 2.0;  // 翻转 Y 轴
    
    output.clip_position = vec4<f32>(normalized_x, normalized_y, 0.0, 1.0);
    output.tex_coords = input.tex_coords;
    
    return output;
}

// ===== 片段着色器 =====

@group(1) @binding(0)
var texture_sampler: sampler;

@group(1) @binding(1)
var texture_view: texture_2d<f32>;

struct FragmentUniforms {
    color: vec4<f32>,       // RGBA 颜色 (0.0-1.0)
    opacity: f32,           // 全局透明度
    grayscale: f32,         // 灰度模式 (0.0 = 关闭, 1.0 = 开启)
    _padding: vec2<f32>,    // 对齐到 16 字节
}

@group(2) @binding(0)
var<uniform> frag_uniforms: FragmentUniforms;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // 采样纹理
    var color = textureSample(texture_view, texture_sampler, input.tex_coords);
    
    // 应用颜色调制
    color = color * frag_uniforms.color;
    
    // 应用全局透明度
    color.a = color.a * frag_uniforms.opacity;
    
    // 灰度效果（对应 C# 的 GrayScalePixelShader）
    if (frag_uniforms.grayscale > 0.5) {
        let gray = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));  // 标准灰度转换
        color = vec4<f32>(gray, gray, gray, color.a);
    }
    
    return color;
}
