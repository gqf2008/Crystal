// Sprite Renderer - 2D 精灵渲染器
// 对应 C# 的 SlimDX.Sprite 功能
//
// C# equivalent: DXManager.Sprite (SlimDX.Direct3D9.Sprite)
//
// C# 使用固定管线渲染，Rust 使用现代可编程管线复刻相同功能

use wgpu;
use bytemuck::{Pod, Zeroable};

/// 精灵顶点
/// 
/// 每个精灵由 4 个顶点（2 个三角形）组成
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],     // 屏幕空间位置 (x, y)
    pub tex_coords: [f32; 2],   // 纹理坐标 (u, v)
}

impl SpriteVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x2,  // tex_coords
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// 顶点着色器 Uniforms
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VertexUniforms {
    pub screen_size: [f32; 2],  // 屏幕尺寸
    pub _padding: [f32; 2],     // 对齐到 16 字节
}

/// 片段着色器 Uniforms
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FragmentUniforms {
    pub color: [f32; 4],        // RGBA 颜色
    pub opacity: f32,           // 全局透明度
    pub grayscale: f32,         // 灰度模式 (0.0 或 1.0)
    pub _padding: [f32; 2],     // 对齐到 16 字节
}

/// 精灵渲染器
/// 
/// C# equivalent: SlimDX.Direct3D9.Sprite
pub struct SpriteRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_uniform_buffer: wgpu::Buffer,
    vertex_bind_group: wgpu::BindGroup,
    fragment_uniform_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    
    // 默认纹理绑定组布局（用于创建纹理绑定组）
    texture_bind_group_layout: wgpu::BindGroupLayout,
    fragment_bind_group_layout: wgpu::BindGroupLayout,
}

impl SpriteRenderer {
    /// 创建精灵渲染器
    /// 
    /// C# equivalent: new Sprite(device)
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // 加载 shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sprite.wgsl").into()),
        });

        // 创建顶点 uniform buffer
        let vertex_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Vertex Uniform Buffer"),
            size: std::mem::size_of::<VertexUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 创建片段 uniform buffer
        let fragment_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Fragment Uniform Buffer"),
            size: std::mem::size_of::<FragmentUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 创建顶点 bind group layout
        let vertex_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Vertex Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // 创建顶点 bind group
        let vertex_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite Vertex Bind Group"),
            layout: &vertex_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // 创建纹理 bind group layout
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        // 创建片段 bind group layout
        let fragment_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Fragment Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // 创建渲染管线
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Render Pipeline Layout"),
            bind_group_layouts: &[
                &vertex_bind_group_layout,      // @group(0)
                &texture_bind_group_layout,     // @group(1)
                &fragment_bind_group_layout,    // @group(2)
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Render Pipeline"),
            layout: Some(&render_pipeline_layout),
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
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),  // 标准 alpha 混合
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,  // 不背面剔除（2D 渲染）
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,  // 2D 不需要深度测试
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // 创建采样器（线性过滤）
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

        Self {
            render_pipeline,
            vertex_uniform_buffer,
            vertex_bind_group,
            fragment_uniform_buffer,
            sampler,
            texture_bind_group_layout,
            fragment_bind_group_layout,
        }
    }

    /// 更新屏幕尺寸
    /// 
    /// 当窗口大小改变时调用
    pub fn update_screen_size(&self, queue: &wgpu::Queue, width: u32, height: u32) {
        let uniforms = VertexUniforms {
            screen_size: [width as f32, height as f32],
            _padding: [0.0, 0.0],
        };
        queue.write_buffer(&self.vertex_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// 更新片段 uniforms（颜色、透明度、灰度）
    pub fn update_fragment_uniforms(
        &self,
        queue: &wgpu::Queue,
        color: [f32; 4],
        opacity: f32,
        grayscale: bool,
    ) {
        let uniforms = FragmentUniforms {
            color,
            opacity,
            grayscale: if grayscale { 1.0 } else { 0.0 },
            _padding: [0.0, 0.0],
        };
        queue.write_buffer(&self.fragment_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// 创建纹理绑定组
    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
            ],
        })
    }

    /// 创建片段绑定组
    fn create_fragment_bind_group(&self, device: &wgpu::Device) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite Fragment Bind Group"),
            layout: &self.fragment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.fragment_uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 绘制精灵
    /// 
    /// C# equivalent: Sprite.Draw(texture, sourceRect, Vector3.Zero, position, color)
    /// 
    /// 参数:
    /// - render_pass: 渲染通道
    /// - device: GPU 设备
    /// - vertex_buffer: 顶点缓冲区
    /// - texture_bind_group: 纹理绑定组
    pub fn draw<'a>(
        &'a self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'a>,
        vertex_buffer: &'a wgpu::Buffer,
        texture_bind_group: &'a wgpu::BindGroup,
        vertex_count: u32,
    ) {
        let fragment_bind_group = self.create_fragment_bind_group(device);
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.vertex_bind_group, &[]);
        render_pass.set_bind_group(1, texture_bind_group, &[]);
        render_pass.set_bind_group(2, &fragment_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_count, 0..1);
    }
}

/// 创建精灵四边形的顶点
/// 
/// C# equivalent: Sprite.Draw() 内部逻辑
/// 
/// 参数:
/// - x, y: 屏幕位置（左上角）
/// - width, height: 精灵大小
/// - src_rect: 源纹理矩形 (x, y, width, height)，None 表示整个纹理
/// - texture_width, texture_height: 纹理实际尺寸
pub fn create_sprite_vertices(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    src_rect: Option<(f32, f32, f32, f32)>,
    texture_width: u32,
    texture_height: u32,
) -> [SpriteVertex; 6] {
    // 计算纹理坐标
    let (u0, v0, u1, v1) = if let Some((sx, sy, sw, sh)) = src_rect {
        // 使用指定的源矩形
        (
            sx / texture_width as f32,
            sy / texture_height as f32,
            (sx + sw) / texture_width as f32,
            (sy + sh) / texture_height as f32,
        )
    } else {
        // 使用整个纹理
        (0.0, 0.0, 1.0, 1.0)
    };

    // 6 个顶点组成 2 个三角形
    [
        // 三角形 1
        SpriteVertex {
            position: [x, y],
            tex_coords: [u0, v0],
        },
        SpriteVertex {
            position: [x + width, y],
            tex_coords: [u1, v0],
        },
        SpriteVertex {
            position: [x, y + height],
            tex_coords: [u0, v1],
        },
        // 三角形 2
        SpriteVertex {
            position: [x + width, y],
            tex_coords: [u1, v0],
        },
        SpriteVertex {
            position: [x + width, y + height],
            tex_coords: [u1, v1],
        },
        SpriteVertex {
            position: [x, y + height],
            tex_coords: [u0, v1],
        },
    ]
}
