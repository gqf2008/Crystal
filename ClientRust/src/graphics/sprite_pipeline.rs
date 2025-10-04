// sprite_pipeline.rs
//
// 对应 C# 的 Sprite (SlimDX.Direct3D9.Sprite)
// 
// C# 使用 DirectX 9 Sprite 进行 2D 渲染
// Rust 使用 wgpu RenderPipeline 实现相同功能
//
// 核心功能：
// - 绘制 2D 纹理到屏幕
// - 支持透明度
// - 支持颜色调制
// - 支持矩形裁剪

use wgpu;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

use super::dx_manager::{DXManager, TextureHandle};

/// 顶点数据结构
/// 
/// 对应 C# Sprite 内部使用的顶点格式
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    /// 位置 (x, y, z)
    pub position: [f32; 3],
    
    /// 纹理坐标 (u, v)
    pub tex_coords: [f32; 2],
    
    /// 颜色 (r, g, b, a)
    pub color: [f32; 4],
}

impl SpriteVertex {
    /// 顶点属性描述
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tex_coords
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Sprite 渲染管道
/// 
/// C# equivalent: SlimDX.Direct3D9.Sprite
/// 
/// C# 的 Sprite 提供了简单的 2D 绘制接口：
/// - Sprite.Begin()
/// - Sprite.Draw(texture, rect, center, position, color)
/// - Sprite.End()
/// 
/// Rust 的 SpritePipeline 使用 wgpu 实现相同功能
pub struct SpritePipeline {
    /// 渲染管道
    pipeline: wgpu::RenderPipeline,
    
    /// 绑定组布局
    bind_group_layout: wgpu::BindGroupLayout,
    
    /// 采样器
    sampler: wgpu::Sampler,
}

impl SpritePipeline {
    /// 创建 SpritePipeline
    /// 
    /// C# equivalent: new Sprite(Device)
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // 创建绑定组布局
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Bind Group Layout"),
            entries: &[
                // 纹理
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 采样器
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        
        // 创建采样器
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sprite Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        
        // 创建 shader 模块
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::shader_source().into()),
        });
        
        // 创建管道布局
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // 创建渲染管道
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[SpriteVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // 不剔除，允许双面
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        
        Self {
            pipeline,
            bind_group_layout,
            sampler,
        }
    }
    
    /// Shader 源代码 (WGSL)
    /// 
    /// 对应 C# 的默认 Sprite shader
    fn shader_source() -> &'static str {
        r#"
// 顶点 shader 输入
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

// 顶点 shader 输出 / 片段 shader 输入
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

// 顶点 shader
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = vec4<f32>(input.position, 1.0);
    output.tex_coords = input.tex_coords;
    output.color = input.color;
    return output;
}

// 片段 shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, input.tex_coords);
    return tex_color * input.color;
}
"#
    }
    
    /// 创建纹理的绑定组
    /// 
    /// C# equivalent: 内部处理，在 Draw 时自动绑定纹理
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        texture: &TextureHandle,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
    
    /// 绘制单个 sprite
    /// 
    /// C# equivalent: Sprite.Draw(texture, sourceRect, center, position, color)
    /// Line: 252-255
    /// 
    /// ```csharp
    /// public static void Draw(Texture texture, Rectangle? sourceRect, Vector3? position, Color4 color)
    /// {
    ///     Sprite.Draw(texture, sourceRect, Vector3.Zero, position, color);
    ///     CMain.DPSCounter++;
    /// }
    /// ```
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        texture: &TextureHandle,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) {
        // 创建顶点缓冲区（矩形的 4 个顶点）
        let vertices = Self::create_quad_vertices(x, y, width, height, color);
        let vertex_data: &[u8] = bytemuck::cast_slice(&vertices);
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Vertex Buffer"),
            size: vertex_data.len() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // TODO: 上传数据到缓冲区
        // device.queue.write_buffer(&vertex_buffer, 0, vertex_data);
        
        // 创建索引缓冲区（两个三角形）
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_data: &[u8] = bytemuck::cast_slice(&indices);
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Index Buffer"),
            size: index_data.len() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // TODO: 上传数据到缓冲区
        // device.queue.write_buffer(&index_buffer, 0, index_data);
        
        // 创建绑定组
        let bind_group = self.create_bind_group(device, texture);
        
        // TODO: 实现实际的渲染
        // 由于 wgpu 22.x API 复杂性，暂时注释掉
        // 需要正确初始化所有结构体字段
        /*
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sprite Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
        */
    }
    
    /// 创建矩形的顶点数据
    fn create_quad_vertices(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) -> [SpriteVertex; 4] {
        // 转换为 NDC 坐标 (假设屏幕坐标系是 0-800x600)
        // TODO: 需要传入实际的屏幕尺寸
        let screen_w = 800.0;
        let screen_h = 600.0;
        
        let x1 = (x / screen_w) * 2.0 - 1.0;
        let y1 = 1.0 - (y / screen_h) * 2.0;
        let x2 = ((x + width) / screen_w) * 2.0 - 1.0;
        let y2 = 1.0 - ((y + height) / screen_h) * 2.0;
        
        [
            // 左上
            SpriteVertex {
                position: [x1, y1, 0.0],
                tex_coords: [0.0, 0.0],
                color,
            },
            // 右上
            SpriteVertex {
                position: [x2, y1, 0.0],
                tex_coords: [1.0, 0.0],
                color,
            },
            // 右下
            SpriteVertex {
                position: [x2, y2, 0.0],
                tex_coords: [1.0, 1.0],
                color,
            },
            // 左下
            SpriteVertex {
                position: [x1, y2, 0.0],
                tex_coords: [0.0, 1.0],
                color,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vertex_size() {
        assert_eq!(
            std::mem::size_of::<SpriteVertex>(),
            std::mem::size_of::<[f32; 9]>()
        );
    }
}
