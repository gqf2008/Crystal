// dx_manager.rs
// 
// 对应 Client/MirGraphics/DXManager.cs
// 
// C# 使用 DirectX 9 (SlimDX)
// Rust 使用 wgpu (跨平台图形API) + winit (窗口管理)
// 保持与 C# 相同的 API 设计

use wgpu;
use wgpu::util::DeviceExt;
use winit::window::Window;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use super::sprite_renderer::SpriteRenderer;

/// 纹理句柄
/// 
/// 对应 C# 的 Texture (SlimDX.Direct3D9.Texture)
pub struct TextureHandle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

/// 混合模式
/// 
/// 对应 C# 的 BlendMode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    InvLight,
}

/// 绘制调用（用于批处理）
/// 
/// 对应 C# 的 Sprite.Draw() 调用参数
#[derive(Clone)]
struct DrawCall {
    texture: Arc<TextureHandle>,
    source_rect: Option<(i32, i32, u32, u32)>,
    position: (f32, f32, f32),
    color: [f32; 4],
}

/// DXManager - 图形设备管理器
/// 
/// C# equivalent: Client.MirGraphics.DXManager (static class)
/// 
/// C# 使用静态类 + DirectX 9
/// Rust 使用实例 + wgpu (更符合 Rust 最佳实践)
/// 
/// 对应的 C# 字段：
/// - static Device Device           → device: wgpu::Device
/// - static Sprite Sprite           → (wgpu 渲染管道)
/// - static float Opacity           → opacity: f32
/// - static bool Blending           → blending: bool
/// - static float BlendingRate      → blending_rate: f32
/// - static bool GrayScale          → grayscale: bool
/// - static List<Texture> Lights    → texture_cache (包含所有纹理)
pub struct DXManager {
    /// wgpu 设备 (对应 C# Device)
    device: Arc<wgpu::Device>,
    
    /// wgpu 队列 (用于提交渲染命令)
    queue: Arc<wgpu::Queue>,
    
    /// 渲染表面 (对应 C# MainSurface)
    surface: Option<wgpu::Surface<'static>>,
    
    /// 表面配置
    surface_config: RefCell<Option<wgpu::SurfaceConfiguration>>,
    
    /// 纹理缓存 (对应 C# TextureList)
    texture_cache: RefCell<HashMap<String, Arc<TextureHandle>>>,
    
    /// 当前全局透明度 (对应 C# Opacity)
    /// C# default: 1.0F
    opacity: RefCell<f32>,
    
    /// 混合模式启用 (对应 C# Blending)
    blending: RefCell<bool>,
    
    /// 混合率 (对应 C# BlendingRate)
    blending_rate: RefCell<f32>,
    
    /// 混合模式类型 (对应 C# BlendingMode)
    blend_mode: RefCell<BlendMode>,
    
    /// 灰度模式 (对应 C# GrayScale)
    grayscale: RefCell<bool>,
    
    /// 屏幕宽度
    screen_width: u32,
    
    /// 屏幕高度
    screen_height: u32,
    
    /// 精灵渲染器 (对应 C# Sprite)
    sprite_renderer: SpriteRenderer,
    
    /// 当前帧的 surface texture (仅在渲染期间有效)
    /// 对应 C# 的 Sprite.Begin() 到 Sprite.End() 之间的状态
    current_frame: RefCell<Option<wgpu::SurfaceTexture>>,
    
    /// 绘制调用队列 (批处理)
    /// 对应 C# Sprite 内部的批处理队列
    draw_queue: RefCell<Vec<DrawCall>>,
}

impl DXManager {
    /// 创建 DXManager 实例
    /// 
    /// C# equivalent: DXManager.Create()
    /// 
    /// ```csharp
    /// public static void Create()
    /// {
    ///     Parameters = new PresentParameters { ... };
    ///     Direct3D d3d = new Direct3D();
    ///     Device = new Device(d3d, ...);
    ///     LoadTextures();
    ///     LoadPixelsShaders();
    /// }
    /// ```
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        
        // 创建 wgpu 实例
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // 创建渲染表面
        let surface = instance.create_surface(window.clone()).ok();
        
        // 请求适配器
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface.as_ref(),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find a suitable adapter");
        
        // 请求设备和队列
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MIR2 Graphics Device"),
            ..Default::default()
        })
        .await
        .expect("Failed to create device");
        
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        // 配置表面
        let surface_config = surface.as_ref().map(|surface| {
            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps.formats.iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_caps.formats[0]);
            
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo, // VSync
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            }
        });
        
        if let (Some(surface), Some(config)) = (&surface, &surface_config) {
            surface.configure(&device, config);
        }
        
        // 创建精灵渲染器
        let surface_format = surface_config.as_ref()
            .map(|config| config.format)
            .unwrap_or(wgpu::TextureFormat::Bgra8UnormSrgb);
        let sprite_renderer = SpriteRenderer::new(&device, surface_format);
        
        Self {
            device,
            queue,
            surface,
            surface_config: RefCell::new(surface_config),
            texture_cache: RefCell::new(HashMap::new()),
            opacity: RefCell::new(1.0),
            blending: RefCell::new(false),
            blending_rate: RefCell::new(1.0),
            blend_mode: RefCell::new(BlendMode::Normal),
            grayscale: RefCell::new(false),
            screen_width: size.width,
            screen_height: size.height,
            sprite_renderer,
            current_frame: RefCell::new(None),
            draw_queue: RefCell::new(Vec::new()),
        }
    }
    
    /// 设置全局透明度
    /// 
    /// C# equivalent: DXManager.SetOpacity(float opacity)
    /// Line: 347
    /// 
    /// ```csharp
    /// public static void SetOpacity(float opacity)
    /// {
    ///     if (Opacity == opacity) return;
    ///     Sprite.Flush();
    ///     Device.SetRenderState(RenderState.AlphaBlendEnable, true);
    ///     // ... 设置混合状态
    ///     Opacity = opacity;
    ///     Sprite.Flush();
    /// }
    /// ```
    pub fn set_opacity(&self, opacity: f32) {
        let current = *self.opacity.borrow();
        if (current - opacity).abs() < 0.001 {
            return;
        }
        
        *self.opacity.borrow_mut() = opacity.clamp(0.0, 1.0);
    }
    
    /// 获取当前透明度
    /// 
    /// C# equivalent: DXManager.Opacity (property getter)
    pub fn opacity(&self) -> f32 {
        *self.opacity.borrow()
    }
    
    /// 设置灰度模式
    /// 
    /// C# equivalent: DXManager.SetGrayscale(bool value)
    /// Line: 234
    /// 
    /// ```csharp
    /// public static void SetGrayscale(bool value)
    /// {
    ///     GrayScale = value;
    ///     if (value == true)
    ///     {
    ///         if (Device.PixelShader == GrayScalePixelShader) return;
    ///         Sprite.Flush();
    ///         Device.PixelShader = GrayScalePixelShader;
    ///     }
    ///     else
    ///     {
    ///         if (Device.PixelShader == null) return;
    ///         Sprite.Flush();
    ///         Device.PixelShader = null;
    ///     }
    /// }
    /// ```
    pub fn set_grayscale(&self, enabled: bool) {
        *self.grayscale.borrow_mut() = enabled;
        // TODO: 切换灰度 shader
    }
    
    /// 获取灰度模式状态
    /// 
    /// C# equivalent: DXManager.GrayScale (property getter)
    pub fn is_grayscale(&self) -> bool {
        *self.grayscale.borrow()
    }
    
    /// 设置混合模式
    /// 
    /// C# equivalent: DXManager.SetBlend(bool value, float rate, BlendMode mode)
    /// Line: 380
    /// 
    /// ```csharp
    /// public static void SetBlend(bool value, float rate = 1F, BlendMode mode = BlendMode.NORMAL)
    /// {
    ///     if (value == Blending && BlendingRate == rate && BlendingMode == mode) return;
    ///     Blending = value;
    ///     BlendingRate = rate;
    ///     BlendingMode = mode;
    ///     
    ///     Sprite.Flush();
    ///     Sprite.End();
    ///     
    ///     if (Blending)
    ///     {
    ///         Sprite.Begin(SpriteFlags.DoNotSaveState);
    ///         Device.SetRenderState(RenderState.AlphaBlendEnable, true);
    ///         // ... 设置混合模式
    ///     }
    ///     else
    ///         Sprite.Begin(SpriteFlags.AlphaBlend);
    /// }
    /// ```
    pub fn set_blend(&self, enabled: bool, rate: f32, mode: BlendMode) {
        let current_blending = *self.blending.borrow();
        let current_rate = *self.blending_rate.borrow();
        let current_mode = *self.blend_mode.borrow();
        
        if enabled == current_blending 
            && (rate - current_rate).abs() < 0.001 
            && mode == current_mode 
        {
            return;
        }
        
        *self.blending.borrow_mut() = enabled;
        *self.blending_rate.borrow_mut() = rate.clamp(0.0, 1.0);
        *self.blend_mode.borrow_mut() = mode;
        
        // TODO: 更新渲染管道的混合状态
    }
    
    /// 获取混合模式状态
    /// 
    /// C# equivalent: DXManager.Blending (property getter)
    pub fn is_blending(&self) -> bool {
        *self.blending.borrow()
    }
    
    /// 加载纹理到 GPU
    /// 
    /// 内部方法，用于将图像数据上传到 wgpu
    /// 
    /// C# equivalent: 内部使用 Texture.FromMemory()
    pub fn load_texture(
        &self,
        label: String,
        width: u32,
        height: u32,
        rgba_data: &[u8],
    ) -> Arc<TextureHandle> {
        // 检查缓存
        {
            let cache = self.texture_cache.borrow();
            if let Some(handle) = cache.get(&label) {
                return handle.clone();
            }
        }
        
        // 创建纹理
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // 上传数据到 GPU
        // C# equivalent: Texture.FromMemory(Device, data, width, height, 1, Usage.None, Format.A8R8G8B8, Pool.Managed, Filter.None, Filter.None, 0)
        // 
            // wgpu 27.0 API 说明:
            // - TexelCopyTextureInfo: 纹理复制目标 (替代旧的 ImageCopyTexture)
            // - TexelCopyBufferLayout: 缓冲区布局 (替代旧的 ImageDataLayout)
            //   - bytes_per_row: Option<u32>
            //   - rows_per_image: Option<u32>
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                rgba_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                size,
            );        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let handle = Arc::new(TextureHandle {
            texture,
            view,
            width,
            height,
        });
        
        // 缓存
        self.texture_cache.borrow_mut().insert(label, handle.clone());
        
        handle
    }
    
    /// 清理纹理缓存
    /// 
    /// C# equivalent: DXManager.Clean()
    /// Line: 436
    /// 
    /// ```csharp
    /// public static void Clean()
    /// {
    ///     for (int i = TextureList.Count - 1; i >= 0; i--)
    ///     {
    ///         MImage m = TextureList[i];
    ///         if (m == null) { TextureList.RemoveAt(i); continue; }
    ///         if (CMain.Time <= m.CleanTime) continue;
    ///         m.DisposeTexture();
    ///     }
    /// }
    /// ```
    pub fn clean_cache(&self) {
        // TODO: 实现基于时间的缓存清理
        // 目前简单清空所有缓存
        self.texture_cache.borrow_mut().clear();
    }
    
    /// 获取 wgpu Device 引用
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    
    /// 获取 wgpu Queue 引用
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    
    /// 调整窗口大小
    /// 
    /// C# equivalent: DXManager.ResetDevice()
    pub fn resize(&self, new_width: u32, new_height: u32) {
        if let Some(ref surface) = self.surface {
            let mut config_opt = self.surface_config.borrow_mut();
            if let Some(ref mut config) = *config_opt {
                config.width = new_width;
                config.height = new_height;
                surface.configure(&self.device, config);
            }
        }
    }
    
    /// 绘制纹理（带透明度）
    /// 
    /// C# equivalent: DXManager.DrawOpaque(Texture texture, Rectangle? sourceRect, Vector3? position, Color4 color, float opacity)
    /// Line: 246-250
    /// 
    /// ```csharp
    /// public static void DrawOpaque(Texture texture, Rectangle? sourceRect, Vector3? position, Color4 color, float opacity)
    /// {
    ///     color.Alpha = opacity;
    ///     Draw(texture, sourceRect, position, color);
    /// }
    /// ```
    pub fn draw_opaque(
        &self,
        texture: &TextureHandle,
        source_rect: Option<(i32, i32, u32, u32)>,  // (x, y, width, height)
        position: Option<(f32, f32, f32)>,           // (x, y, z)
        color: [f32; 4],                             // RGBA
        opacity: f32,
    ) {
        let mut color_with_opacity = color;
        color_with_opacity[3] = opacity;
        self.draw(texture, source_rect, position, color_with_opacity);
    }
    
    /// 绘制纹理（批处理模式）
    /// 
    /// 添加绘制命令到批处理队列，在 end_frame() 时统一执行
    /// 
    /// C# equivalent: DXManager.Draw(Texture texture, Rectangle? sourceRect, Vector3? position, Color4 color)
    /// Line: 252-256
    /// 
    /// ```csharp
    /// public static void Draw(Texture texture, Rectangle? sourceRect, Vector3? position, Color4 color)
    /// {
    ///     Sprite.Draw(texture, sourceRect, Vector3.Zero, position, color);
    ///     CMain.DPSCounter++;
    /// }
    /// ```
    /// 
    /// 注意：必须在 begin_frame() 和 end_frame() 之间调用
    /// 
    /// 参数说明：
    /// - texture: 纹理句柄
    /// - source_rect: 源矩形区域 (x, y, width, height)，None 表示整个纹理
    /// - position: 屏幕位置 (x, y, z)，None 表示 (0, 0, 0)
    /// - color: RGBA 颜色值 [r, g, b, a]，范围 0.0-1.0
    pub fn draw(
        &self,
        texture: &TextureHandle,
        source_rect: Option<(i32, i32, u32, u32)>,
        position: Option<(f32, f32, f32)>,
        color: [f32; 4],
    ) {
        // 添加到批处理队列
        self.draw_queue.borrow_mut().push(DrawCall {
            texture: Arc::new(TextureHandle {
                texture: texture.texture.clone(),  // wgpu::Texture 不支持 clone，需要使用 Arc
                view: texture.view.clone(),        // wgpu::TextureView 支持 clone
                width: texture.width,
                height: texture.height,
            }),
            source_rect,
            position: position.unwrap_or((0.0, 0.0, 0.0)),
            color,
        });
        
        // TODO: 计数器 (对应 CMain.DPSCounter++)
    }
    
    /// 开始渲染帧（获取 surface texture 并清空屏幕）
    /// 
    /// C# equivalent: Device.BeginScene() + Device.Clear() + Sprite.Begin()
    /// 
    /// ```csharp
    /// Device.BeginScene();
    /// Device.Clear(ClearFlags.Target | ClearFlags.ZBuffer, Color.Black, 1.0f, 0);
    /// Sprite.Begin(SpriteFlags.AlphaBlend);
    /// ```
    pub fn begin_frame(&self, clear_color: [f32; 4]) {
        // 清空批处理队列
        self.draw_queue.borrow_mut().clear();
        
        // 获取 surface texture
        let surface = match self.surface.as_ref() {
            Some(s) => s,
            None => return,
        };
        
        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Failed to get surface texture: {:?}", e);
                return;
            }
        };
        
        // 清空屏幕
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Encoder"),
        });
        
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // 更新渲染器屏幕尺寸
        self.sprite_renderer.update_screen_size(&self.queue, self.screen_width, self.screen_height);
        
        // 存储当前帧
        *self.current_frame.borrow_mut() = Some(frame);
    }
    
    /// 结束渲染帧（执行所有绘制命令并 present）
    /// 
    /// C# equivalent: Sprite.End() + Device.EndScene() + Device.Present()
    /// 
    /// ```csharp
    /// Sprite.End();
    /// Device.EndScene();
    /// Device.Present();
    /// ```
    pub fn end_frame(&self) {
        // 获取当前帧
        let frame = match self.current_frame.borrow_mut().take() {
            Some(f) => f,
            None => {
                tracing::warn!("end_frame() called without begin_frame()");
                return;
            }
        };
        
        // 如果没有绘制命令，直接 present
        let draw_queue = self.draw_queue.borrow();
        if draw_queue.is_empty() {
            frame.present();
            return;
        }
        
        // 准备所有绘制资源（在 render_pass 之外创建以避免生命周期问题）
        let mut draw_resources = Vec::new();
        
        for draw_call in draw_queue.iter() {
            let texture = &draw_call.texture;
            let source_rect = draw_call.source_rect;
            let (pos_x, pos_y, _pos_z) = draw_call.position;
            let color = draw_call.color;
            
            // 计算绘制参数
            let src_rect = source_rect.map(|(x, y, w, h)| (x as f32, y as f32, w as f32, h as f32));
            
            let (width, height) = if let Some((_, _, w, h)) = src_rect {
                (w, h)
            } else {
                (texture.width as f32, texture.height as f32)
            };
            
            // 创建顶点数据
            let vertices = super::sprite_renderer::create_sprite_vertices(
                pos_x,
                pos_y,
                width,
                height,
                src_rect,
                texture.width,
                texture.height,
            );
            
            // 创建顶点缓冲区
            let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Sprite Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            
            // 更新片段着色器 uniforms
            self.sprite_renderer.update_fragment_uniforms(
                &self.queue,
                color,
                *self.opacity.borrow(),
                *self.grayscale.borrow(),
            );
            
            // 创建纹理绑定组
            let texture_bind_group = self.sprite_renderer.create_texture_bind_group(&self.device, &texture.view);
            
            draw_resources.push((vertex_buffer, texture_bind_group, vertices.len()));
        }
        
        // 执行所有绘制命令
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Batch Draw Encoder"),
        });
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Batch Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,  // 保留清屏后的内容
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            // 绘制所有精灵
            for (vertex_buffer, texture_bind_group, vertex_count) in &draw_resources {
                self.sprite_renderer.draw(
                    &self.device,
                    &mut render_pass,
                    vertex_buffer,
                    texture_bind_group,
                    *vertex_count as u32,
                );
            }
        }
        
        // 提交命令
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // 呈现帧
        frame.present();
    }
    
    /// 获取窗口尺寸
    /// 
    /// 返回: (width, height)
    pub fn window_size(&self) -> (u32, u32) {
        (self.screen_width, self.screen_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_blend_mode() {
        assert_eq!(BlendMode::Normal, BlendMode::Normal);
        assert_ne!(BlendMode::Normal, BlendMode::InvLight);
    }
}
