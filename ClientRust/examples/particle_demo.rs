// examples/particle_demo.rs
// 
// 粒子系统演示程序
// 
// 演示内容:
// - 加载 Weather.lib 纹理库
// - 创建粒子引擎（雪花效果）
// - 渲染粒子到屏幕

use mir2_client::graphics::{
    DXManager,
    LibraryName,
    load_library,
    set_data_path,
    ParticleEngine, ParticleImageInfo, ParticleType,
};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Crystal 粒子系统演示 ===");
    println!("按 ESC 退出");
    
    // 1. 创建窗口
    let event_loop = EventLoop::new()?;
    
    #[allow(deprecated)]
    let window = event_loop.create_window(
        winit::window::WindowAttributes::default()
            .with_title("Crystal - Particle Demo (Press ESC to exit)")
            .with_inner_size(winit::dpi::LogicalSize::new(SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64))
    )?;
    
    let window = std::sync::Arc::new(window);
    
    // 2. 初始化 DXManager
    println!("初始化图形设备...");
    let mut dx_manager = pollster::block_on(DXManager::new(window.clone()));
    
    // 确保DXManager的大小和窗口大小一致
    let current_size = window.inner_size();
    println!("当前窗口大小: {}x{}", current_size.width, current_size.height);
    if current_size.width != SCREEN_WIDTH || current_size.height != SCREEN_HEIGHT {
        println!("调整 DXManager 大小以匹配窗口");
        dx_manager.resize(current_size.width, current_size.height);
    }
    
    // 3. 加载纹理库（使用全局库管理器）
    println!("初始化全局库管理器...");
    set_data_path("Data");
    
    println!("加载 Weather.lib...");
    match load_library(LibraryName::Weather) {
        Ok(_) => {
            println!("✓ 成功加载 Weather 库");
        }
        Err(e) => {
            eprintln!("✗ 无法加载 Weather 库: {}", e);
            eprintln!("请确保 Data/Weather.lib 文件存在");
            return Err(e.into());
        }
    }
    
    // 4. 创建粒子纹理信息
    // Weather.lib 索引说明:
    // 索引 0: 512×512 大图 (太大,跳过)
    // 索引 1-9: 32×32 小图 (适合作为粒子纹理)
    let textures = vec![
        ParticleImageInfo::new(LibraryName::Weather, 1, 1, 50),
        ParticleImageInfo::new(LibraryName::Weather, 2, 1, 50),
        ParticleImageInfo::new(LibraryName::Weather, 3, 1, 50),
        ParticleImageInfo::new(LibraryName::Weather, 4, 1, 50),
        ParticleImageInfo::new(LibraryName::Weather, 5, 1, 50),
    ];
    
    // 5. 创建粒子引擎 (雪花效果)
    println!("创建雪花粒子引擎...");
    let mut particle_engine = ParticleEngine::new(
        textures,
        (0.0, 0.0), // emitter location
        ParticleType::Snow,
        SCREEN_WIDTH as i32,
        SCREEN_HEIGHT as i32,
    );
    
    println!("✓ 粒子系统初始化完成");
    println!("\n开始渲染...");
    
    // 帧计数器
    let mut frame_count = 0u64;
    let start_time = std::time::Instant::now();
    let mut last_fps_time = start_time;
    
    // 保持window引用以便请求重绘
    let window_ref = window.clone();
    
    // 跟踪当前大小,避免重复resize
    let mut current_size = (SCREEN_WIDTH, SCREEN_HEIGHT);
    
    // 6. 主循环
    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    println!("\n程序退出");
                    elwt.exit();
                }
                
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.physical_key == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Escape) {
                        println!("\n用户按下 ESC，退出");
                        elwt.exit();
                    }
                }
                
                WindowEvent::Resized(new_size) => {
                    let new_size_tuple = (new_size.width, new_size.height);
                    if new_size_tuple != current_size {
                        println!("窗口大小改变: {}x{} -> {}x{}", current_size.0, current_size.1, new_size.width, new_size.height);
                        dx_manager.resize(new_size.width, new_size.height);
                        current_size = new_size_tuple;
                    }
                }
                
                WindowEvent::RedrawRequested => {
                    // 更新粒子
                    particle_engine.process();
                    
                    // 渲染（不再需要传递 library 参数）
                    if let Err(e) = render_frame(
                        &mut particle_engine,
                        &mut dx_manager,
                    ) {
                        eprintln!("渲染错误: {}", e);
                    }
                    
                    // FPS 统计
                    frame_count += 1;
                    let elapsed = last_fps_time.elapsed();
                    if elapsed.as_secs() >= 1 {
                        let fps = frame_count as f64 / elapsed.as_secs_f64();
                        println!(
                            "FPS: {:.1} | 粒子数: {} | 运行时间: {:.1}s",
                            fps,
                            particle_engine.particle_count(),
                            start_time.elapsed().as_secs_f64()
                        );
                        frame_count = 0;
                        last_fps_time = std::time::Instant::now();
                    }
                }
                
                _ => {}
            },
            
            Event::AboutToWait => {
                // 请求重绘
                window_ref.request_redraw();
            }
            
            _ => {}
        }
    })?;
    
    Ok(())
}

/// 渲染一帧
fn render_frame(
    particle_engine: &mut ParticleEngine,
    dx_manager: &mut DXManager,
) -> Result<(), Box<dyn std::error::Error>> {
    // 开始帧（清屏为黑色）
    dx_manager.begin_frame([0.0, 0.0, 0.0, 1.0]);
    
    // 绘制所有粒子（自动从全局库管理器获取库）
    particle_engine.draw(
        dx_manager,
        SCREEN_WIDTH as i32,
        SCREEN_HEIGHT as i32,
    )?;
    
    // 结束帧
    dx_manager.end_frame();
    
    Ok(())
}
