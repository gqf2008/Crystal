// MLibrary + Ggez 集成示例
// 
// 演示如何将 MLibrary 加载的 .lib 图片数据集成到 ggez 渲染
// 
// 运行: cargo run --example mlibrary_ggez_example
//
// 功能:
// 1. 加载 Data.lib (游戏核心图库)
// 2. 提取图片像素数据
// 3. 创建 ggez Image
// 4. 渲染精灵

use anyhow::Result;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::conf::{WindowMode, WindowSetup};
use ggez::graphics::{self, Color, Image};
use std::path::PathBuf;

// 假设已经实现了 MLibrary
// use mir2_client::graphics::{MLibrary, ImageInfo};

fn main() -> Result<()> {
    println!("=== MLibrary + Ggez 集成示例 ===");
    println!("注意: 需要 Data/ 目录下有 Data.lib 文件");
    
    // 创建 ggez Context
    let (mut ctx, event_loop) = ContextBuilder::new("mlibrary_ggez", "Crystal")
        .window_setup(WindowSetup::default().title("MLibrary + Ggez 示例"))
        .window_mode(WindowMode::default().dimensions(800.0, 600.0))
        .build()?;
    
    // 创建游戏状态
    let mut game = MLibraryExample::new(&mut ctx)?;
    
    // 运行事件循环
    event::run(ctx, event_loop, game)
        .map_err(|e| anyhow::anyhow!("运行错误: {}", e))
}

struct MLibraryExample {
    // mlibrary: MLibrary,  // MLibrary 实例
    loaded_images: Vec<Image>,  // 加载的图片
    current_index: usize,       // 当前显示的图片索引
    total_images: usize,        // 总图片数
    error_message: Option<String>,
}

impl MLibraryExample {
    fn new(ctx: &mut Context) -> Result<Self> {
        println!("初始化 MLibrary...");
        
        // TODO: 实际加载 MLibrary
        // let lib_path = PathBuf::from("Data/Data.lib");
        // let mlibrary = MLibrary::load(&lib_path)?;
        // let total_images = mlibrary.image_count();
        
        // 临时: 创建测试图片
        let test_images = vec![
            create_test_image(ctx, 64, 64, (255, 100, 100))?,  // 红色
            create_test_image(ctx, 64, 64, (100, 255, 100))?,  // 绿色
            create_test_image(ctx, 64, 64, (100, 100, 255))?,  // 蓝色
        ];
        
        println!("MLibrary 初始化完成 (使用测试图片)");
        
        Ok(Self {
            // mlibrary,
            loaded_images: test_images,
            current_index: 0,
            total_images: 3,
            error_message: Some("MLibrary 集成待完成 - 当前显示测试图片".to_string()),
        })
    }
    
    fn load_image_from_mlibrary(&mut self, ctx: &mut Context, index: usize) -> Result<Image> {
        // TODO: 实际实现
        // let image_info = self.mlibrary.get_image_info(index)?;
        // let pixels = self.mlibrary.get_image_pixels(index)?;
        // 
        // let image = Image::from_rgba8(
        //     ctx,
        //     image_info.width,
        //     image_info.height,
        //     &pixels
        // )?;
        
        // 临时: 返回测试图片
        create_test_image(ctx, 64, 64, (255, 255, 255))
            .map_err(|e| anyhow::anyhow!("创建测试图片失败: {}", e))
    }
}

impl EventHandler for MLibraryExample {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 检查键盘输入
        if ctx.keyboard.is_key_just_pressed(ggez::input::keyboard::KeyCode::Right) {
            self.current_index = (self.current_index + 1) % self.total_images;
            println!("切换到图片 {}/{}", self.current_index + 1, self.total_images);
        }
        
        if ctx.keyboard.is_key_just_pressed(ggez::input::keyboard::KeyCode::Left) {
            self.current_index = if self.current_index == 0 {
                self.total_images - 1
            } else {
                self.current_index - 1
            };
            println!("切换到图片 {}/{}", self.current_index + 1, self.total_images);
        }
        
        if ctx.keyboard.is_key_pressed(ggez::input::keyboard::KeyCode::Escape) {
            ctx.request_quit();
        }
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::from_rgb(40, 40, 60));
        
        // 绘制当前图片 (居中)
        if let Some(image) = self.loaded_images.get(self.current_index) {
            let x = 400.0 - (image.width() as f32 / 2.0);
            let y = 300.0 - (image.height() as f32 / 2.0);
            
            canvas.draw(
                image,
                graphics::DrawParam::default()
                    .dest([x, y])
                    .scale([4.0, 4.0])  // 放大4倍
            );
        }
        
        // 绘制信息文本
        let info_text = graphics::Text::new(format!(
            "MLibrary + Ggez 集成示例\n\n\
             图片: {}/{}\n\
             分辨率: {}x{}\n\n\
             控制:\n\
             ← → : 切换图片\n\
             ESC : 退出\n\n\
             {}",
            self.current_index + 1,
            self.total_images,
            self.loaded_images.get(self.current_index)
                .map(|img| img.width()).unwrap_or(0),
            self.loaded_images.get(self.current_index)
                .map(|img| img.height()).unwrap_or(0),
            self.error_message.as_deref().unwrap_or(""),
        ));
        
        canvas.draw(
            &info_text,
            graphics::DrawParam::default()
                .dest([20.0, 20.0])
                .color(Color::WHITE)
        );
        
        // 绘制集成代码示例
        let code_text = graphics::Text::new(
            "集成代码示例:\n\
             \n\
             // 1. 加载 MLibrary\n\
             let lib = MLibrary::load(\"Data.lib\")?;\n\
             \n\
             // 2. 获取图片数据\n\
             let pixels = lib.get_image_pixels(index)?;\n\
             \n\
             // 3. 创建 ggez Image\n\
             let image = Image::from_rgba8(\n\
             \x20   ctx, width, height, &pixels\n\
             )?;\n\
             \n\
             // 4. 渲染\n\
             canvas.draw(&image, [x, y]);"
        );
        
        canvas.draw(
            &code_text,
            graphics::DrawParam::default()
                .dest([20.0, 400.0])
                .color(Color::from_rgb(200, 200, 150))
        );
        
        canvas.finish(ctx)?;
        Ok(())
    }
}

/// 创建测试图片 (纯色 + 渐变边框)
fn create_test_image(ctx: &mut Context, width: u16, height: u16, color: (u8, u8, u8)) -> GameResult<Image> {
    let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 4);
    
    for y in 0..height {
        for x in 0..width {
            // 边框渐变效果
            let border_dist = x.min(y).min(width - 1 - x).min(height - 1 - y) as f32;
            let border_factor = (border_dist / 8.0).min(1.0);
            
            let r = (color.0 as f32 * border_factor) as u8;
            let g = (color.1 as f32 * border_factor) as u8;
            let b = (color.2 as f32 * border_factor) as u8;
            let a = 255u8;
            
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }
    
    Image::from_rgba8(ctx, width, height, &pixels)
}
