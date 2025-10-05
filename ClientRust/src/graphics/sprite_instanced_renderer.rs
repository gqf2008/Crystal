// Sprite Instanced Renderer - GPU实例化精灵渲染器
// 用于高性能批量渲染大量相同纹理的精灵

use wgpu;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

/// Quad顶点 (所有实例共享的模板)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],     // 局部坐标 (0,0 到 1,1)
    pub tex_coords: [f32; 2],   // 纹理坐标
}

impl QuadVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,  // position
        1 => Float32x2,  // tex_coords
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// 实例数据 (每个精灵独立的数据)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SpriteInstance {
    pub position: [f32; 2],     // 世界坐标 (像素)
    pub size: [f32; 2],         // 尺寸 (像素)
    pub color: [f32; 4],        // RGBA颜色
}

impl SpriteInstance {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        2 => Float32x2,  // instance_position
        3 => Float32x2,  // instance_size
        4 => Float32x4,  // instance_color
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// 顶点着色器 Uniforms (复用sprite_renderer.rs的)
use super::sprite_renderer::{VertexUniforms, FragmentUniforms};

/// GPU实例化精灵渲染器
pub struct SpriteInstancedRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_uniform_buffer: wgpu::Buffer,
    vertex_bind_group: wgpu::BindGroup,
    fragment_uniform_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    
    texture_bind_group_layout: wgpu::BindGroupLayout,
    fragment_bind_group_layout: wgpu::BindGroupLayout,
    
    // Quad模板缓冲区 (固定的6个顶点)
    quad_vertex_buffer: wgpu::Buffer,
}

impl SpriteInstancedRenderer {
    /// 创建实例化渲染器
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // 加载实例化shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Instanced Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/sprite_instanced.wgsl").into()),
        });

        // 创建顶点 uniform buffer
        let vertex_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Instanced Vertex Uniform Buffer"),
            size: std::mem::size_of::<VertexUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 创建片段 uniform buffer
        let fragment_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Instanced Fragment Uniform Buffer"),
            size: std::mem::size_of::<FragmentUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 创建顶点 bind group layout
        let vertex_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite Instanced Vertex Bind Group Layout"),
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
            label: Some("Sprite Instanced Vertex Bind Group"),
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
            label: Some("Sprite Instanced Texture Bind Group Layout"),
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
            label: Some("Sprite Instanced Fragment Bind Group Layout"),
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
            label: Some("Sprite Instanced Render Pipeline Layout"),
            bind_group_layouts: &[
                &vertex_bind_group_layout,      // @group(0)
                &texture_bind_group_layout,     // @group(1)
                &fragment_bind_group_layout,    // @group(2)
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Instanced Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    QuadVertex::desc(),         // Quad模板顶点
                    SpriteInstance::desc(),     // 实例数据
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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

        // 创建采样器
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sprite Instanced Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 创建Quad模板顶点 (0,0 到 1,1 的矩形)
        let quad_vertices = [
            // 三角形 1
            QuadVertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
            QuadVertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
            QuadVertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
            // 三角形 2
            QuadVertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
            QuadVertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
            QuadVertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
        ];

        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Instanced Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            render_pipeline,
            vertex_uniform_buffer,
            vertex_bind_group,
            fragment_uniform_buffer,
            sampler,
            texture_bind_group_layout,
            fragment_bind_group_layout,
            quad_vertex_buffer,
        }
    }

    /// 更新屏幕尺寸
    pub fn update_screen_size(&self, queue: &wgpu::Queue, width: u32, height: u32) {
        let uniforms = VertexUniforms {
            screen_size: [width as f32, height as f32],
            _padding: [0.0, 0.0],
        };
        queue.write_buffer(&self.vertex_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// 更新片段 uniforms
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
            label: Some("Sprite Instanced Texture Bind Group"),
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
            label: Some("Sprite Instanced Fragment Bind Group"),
            layout: &self.fragment_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.fragment_uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// 绘制实例化精灵
    /// 
    /// 参数:
    /// - render_pass: 渲染通道
    /// - device: GPU设备
    /// - instance_buffer: 实例数据缓冲区
    /// - texture_bind_group: 纹理绑定组
    /// - instance_count: 实例数量
    pub fn draw_instanced<'a>(
        &'a self,
        device: &wgpu::Device,
        render_pass: &mut wgpu::RenderPass<'a>,
        instance_buffer: &'a wgpu::Buffer,
        texture_bind_group: &'a wgpu::BindGroup,
        instance_count: u32,
    ) {
        let fragment_bind_group = self.create_fragment_bind_group(device);
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.vertex_bind_group, &[]);
        render_pass.set_bind_group(1, texture_bind_group, &[]);
        render_pass.set_bind_group(2, &fragment_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.draw(0..6, 0..instance_count);  // 6个顶点,instance_count个实例
    }
}
