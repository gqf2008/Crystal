// dx_manager.rs
// 
// 对应 Client/MirGraphics/DXManager.cs
// 
// C# 使用 DirectX 9 (SlimDX)
// Rust 使用 wgpu (跨平台图形API) + winit (窗口管理)
// 保持与 C# 相同的 API 设计

use wgpu;
use winit::window::Window;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

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
