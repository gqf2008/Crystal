//! 测试精灵渲染管道
//! 
//! 这个示例演示如何使用 DXManager 和 SpriteRenderer 绘制 2D 精灵
//! 
//! 运行方式:
//! ```bash
//! cargo run --example test_sprite_rendering
//! ```

use mir2_client::graphics::{DXManager, MLibrary};
use std::sync::Arc;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    tracing::info!("=== 精灵渲染测试 ===");
    
    // 创建窗口
    let event_loop = EventLoop::new()?;
    
    #[allow(deprecated)]
    let window = event_loop.create_window(
        winit::window::WindowAttributes::default()
            .with_title("Sprite Rendering Test - Press ESC to exit")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0))
    )?;
    
    let window_arc = Arc::new(window);
    
    // 创建 DXManager
    tracing::info!("初始化 wgpu 渲染器...");
    let dx_manager = pollster::block_on(DXManager::new(window_arc.clone()));
    
    tracing::info!("渲染器初始化完成");
    
    // 尝试加载 .lib 文件（如果存在）
    let lib_path = "Data/Prguse.lib";
    let texture_handle = if std::path::Path::new(lib_path).exists() {
        tracing::info!("找到 .lib 文件，加载纹理...");
        
        match MLibrary::open(lib_path) {
            Ok(mut lib) => {
                match lib.load_rgba_data(0) {
                    Ok((info, rgba_data)) => {
                        tracing::info!(
                            "加载纹理成功: {}x{}, {} bytes",
                            info.width, info.height, rgba_data.len()
                        );
                        
                        Some(dx_manager.load_texture(
                            "test_sprite".to_string(),
                            info.width as u32,
                            info.height as u32,
                            &rgba_data,
                        ))
                    }
                    Err(e) => {
                        tracing::warn!("加载纹理失败: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("打开 .lib 文件失败: {}", e);
                None
            }
        }
    } else {
        tracing::info!("未找到 .lib 文件，创建测试纹理...");
        None
    };
    
    // 如果没有 .lib 文件，创建一个简单的测试纹理（红色方块）
    let texture_handle = texture_handle.unwrap_or_else(|| {
        tracing::info!("创建 256x256 红色测试纹理");
        let width = 256;
        let height = 256;
        let mut rgba_data = vec![0u8; (width * height * 4) as usize];
        
        // 创建一个红色方块，带半透明边缘
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                
                // 计算到中心的距离
                let cx = x as f32 - width as f32 / 2.0;
                let cy = y as f32 - height as f32 / 2.0;
                let dist = (cx * cx + cy * cy).sqrt();
                let max_dist = (width as f32 / 2.0).min(height as f32 / 2.0);
                
                // 边缘渐变透明
                let alpha = if dist < max_dist * 0.8 {
                    255
                } else {
                    ((1.0 - (dist - max_dist * 0.8) / (max_dist * 0.2)) * 255.0) as u8
                };
                
                rgba_data[idx] = 255;     // R
                rgba_data[idx + 1] = 0;   // G
                rgba_data[idx + 2] = 0;   // B
                rgba_data[idx + 3] = alpha; // A
            }
        }
        
        dx_manager.load_texture("test_sprite".to_string(), width, height, &rgba_data)
    });
    
    tracing::info!("纹理加载完成，开始渲染循环");
    tracing::info!("按 ESC 退出");
    
    // 动画参数
    let mut frame_count = 0u32;
    let mut last_fps_time = std::time::Instant::now();
    let mut fps_counter = 0u32;
    let mut current_fps = 0u32;
    
    // 运行事件循环
    #[allow(deprecated)]
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                tracing::info!("窗口关闭");
                elwt.exit();
            }
            
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                    ..
                },
                ..
            } => {
                tracing::info!("按下 ESC，退出");
                elwt.exit();
            }
            
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                tracing::info!("窗口大小改变: {}x{}", new_size.width, new_size.height);
                dx_manager.resize(new_size.width, new_size.height);
            }
            
            Event::AboutToWait => {
                window_arc.request_redraw();
            }
            
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // 渲染帧
                render_frame(&dx_manager, &texture_handle, frame_count);
                frame_count += 1;
                
                // 计算 FPS
                fps_counter += 1;
                let now = std::time::Instant::now();
                if now.duration_since(last_fps_time).as_secs() >= 1 {
                    current_fps = fps_counter;
                    fps_counter = 0;
                    last_fps_time = now;
                    tracing::info!("FPS: {}, 总帧数: {}", current_fps, frame_count);
                }
            }
            
            _ => {}
        }
    })?;
    
    Ok(())
}

/// 渲染一帧
fn render_frame(
    dx_manager: &DXManager,
    texture: &mir2_client::graphics::TextureHandle,
    frame_count: u32,
) {
    // 获取窗口尺寸
    let window_size = dx_manager.window_size();
    
    // 计算动画位置
    let time = frame_count as f32 * 0.05;
    let radius = 150.0;
    let center_x = 400.0;
    let center_y = 300.0;
    let x = center_x + time.cos() * radius;
    let y = center_y + time.sin() * radius;
    
    // 开始渲染帧（清空屏幕）
    dx_manager.begin_frame([0.0, 0.1, 0.2, 1.0]);
    
    // 绘制多个精灵（现在支持了！）
    
    // 测试 1: 移动的精灵 - 圆周运动
    dx_manager.draw(
        texture,
        None,
        Some((x, y, 0.0)),
        [1.0, 1.0, 1.0, 1.0], // 白色，不透明
    );
    
    // 测试 2: 静态精灵 - 左上角
    dx_manager.draw(
        texture,
        None,
        Some((50.0, 50.0, 0.0)),
        [1.0, 1.0, 1.0, 0.8], // 白色，80% 不透明
    );
    
    // 测试 3: 颜色调制 - 红色
    dx_manager.draw(
        texture,
        None,
        Some((window_size.0 as f32 - 300.0, 50.0, 0.0)),
        [1.0, 0.0, 0.0, 1.0], // 红色
    );
    
    // 测试 4: 颜色调制 - 绿色
    dx_manager.draw(
        texture,
        None,
        Some((window_size.0 as f32 - 300.0, 200.0, 0.0)),
        [0.0, 1.0, 0.0, 1.0], // 绿色
    );
    
    // 测试 5: 颜色调制 - 蓝色
    dx_manager.draw(
        texture,
        None,
        Some((window_size.0 as f32 - 300.0, 350.0, 0.0)),
        [0.0, 0.0, 1.0, 1.0], // 蓝色
    );
    
    // 测试 6: 半透明精灵 - 淡入淡出
    let alpha = ((time * 2.0).sin() * 0.5 + 0.5) * 0.7 + 0.3; // 0.3 ~ 1.0
    dx_manager.draw(
        texture,
        None,
        Some((50.0, window_size.1 as f32 - 300.0, 0.0)),
        [1.0, 1.0, 0.0, alpha], // 黄色，动态透明度
    );
    
    // 结束帧（执行所有绘制并 present）
    dx_manager.end_frame();
}
