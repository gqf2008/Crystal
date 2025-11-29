// ============================================================================
// 测试程序：macroquad 原生 UI 背包对话框（多窗口+标签页测试）
// ============================================================================
//
// 快捷键：
//   I      - 打开/关闭背包1
//   O      - 打开/关闭背包2
//   P      - 打开/关闭背包3
//   1/2/3  - 切换标签页（装备/道具/任务）
//   ESC    - 退出程序
//
// 操作说明：
//   - 点击标签页切换不同物品分类
//   - 左键点击物品开始拖动
//   - 拖动到其他格子释放完成交换
//   - 右键取消拖动
//   - 鼠标滚轮上下滚动
//   - 拖动标题栏移动窗口
//   - 点击窗口将其置顶
//
// ============================================================================

use macroquad::prelude::*;

// 引用项目模块
use client_macroquad::resources::{set_data_path, preload_libraries, LibraryName};
use client_macroquad::scenes::dialogs::game::InventoryDialogNative;

fn window_conf() -> Conf {
    Conf {
        window_title: "Macroquad Native UI - Multi Window Test".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: true,
        ..Default::default()
    }
}

/// 窗口管理器 - 管理多个窗口的绘制顺序和焦点
struct WindowManager {
    /// 窗口列表（按绘制顺序，最后一个在最上层）
    dialogs: Vec<InventoryDialogNative>,
    /// 当前焦点窗口索引
    focused_index: Option<usize>,
}

impl WindowManager {
    fn new() -> Self {
        Self {
            dialogs: Vec::new(),
            focused_index: None,
        }
    }
    
    fn add_dialog(&mut self, mut dialog: InventoryDialogNative, pos: Vec2) {
        dialog.set_position(pos);
        self.dialogs.push(dialog);
    }
    
    /// 将指定窗口置顶
    fn bring_to_front(&mut self, index: usize) {
        if index < self.dialogs.len() && Some(index) != self.focused_index {
            let dialog = self.dialogs.remove(index);
            self.dialogs.push(dialog);
            self.focused_index = Some(self.dialogs.len() - 1);
            println!("🔝 窗口 {} 置顶", index);
        }
    }
    
    /// 检查点击并处理焦点
    fn handle_click(&mut self, pos: Vec2) {
        // 从最上层开始检查
        for i in (0..self.dialogs.len()).rev() {
            if self.dialogs[i].contains(pos) {
                self.bring_to_front(i);
                break;
            }
        }
    }
    
    /// 更新和绘制所有窗口
    fn update_and_draw(&mut self) {
        // 按顺序绘制（最后绘制的在最上层）
        for dialog in &mut self.dialogs {
            dialog.update_and_draw();
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("===========================================");
    println!("  macroquad 原生 UI 多窗口+标签页测试");
    println!("===========================================");
    println!("快捷键:");
    println!("  I     - 打开/关闭背包1");
    println!("  O     - 打开/关闭背包2");
    println!("  P     - 打开/关闭背包3");
    println!("  1/2/3 - 切换标签页（装备/道具/任务）");
    println!("  ESC   - 退出程序");
    println!("");
    println!("操作说明:");
    println!("  - 点击标签页切换物品分类");
    println!("  - 点击窗口将其置顶");
    println!("  - 拖动标题栏移动窗口");
    println!("  - 左键拖动物品交换");
    println!("  - 鼠标滚轮上下滚动");
    println!("===========================================");
    
    // 初始化资源路径
    println!("🔄 正在加载纹理资源...");
    set_data_path("./Data/");
    preload_libraries(&[
        LibraryName::Title,
        LibraryName::Prguse,
        LibraryName::Prguse2,
        LibraryName::Items,
    ]);
    println!("✅ 纹理库加载完成");
    
    // 创建窗口管理器
    let mut window_manager = WindowManager::new();
    
    // 创建3个背包窗口
    for i in 0..3 {
        let mut dialog = InventoryDialogNative::new();
        dialog.load_textures().await;
        dialog.open();
        // 错开位置
        let pos = vec2(50.0 + i as f32 * 100.0, 50.0 + i as f32 * 80.0);
        window_manager.add_dialog(dialog, pos);
    }
    
    println!("📦 创建了3个背包窗口");
    
    loop {
        clear_background(Color::from_rgba(30, 30, 40, 255));
        
        // 绘制背景网格
        let grid_color = Color::from_rgba(50, 50, 60, 100);
        for x in (0..screen_width() as i32).step_by(50) {
            draw_line(x as f32, 0.0, x as f32, screen_height(), 1.0, grid_color);
        }
        for y in (0..screen_height() as i32).step_by(50) {
            draw_line(0.0, y as f32, screen_width(), y as f32, 1.0, grid_color);
        }
        
        // 绘制说明文字
        draw_text(
            "[I/O/P] Toggle windows | [1/2/3] Switch tabs | Click tabs or use shortcuts",
            10.0, 30.0, 18.0, WHITE
        );
        
        // 处理快捷键
        if is_key_pressed(KeyCode::I) && window_manager.dialogs.len() > 0 {
            window_manager.dialogs[0].toggle();
        }
        if is_key_pressed(KeyCode::O) && window_manager.dialogs.len() > 1 {
            window_manager.dialogs[1].toggle();
        }
        if is_key_pressed(KeyCode::P) && window_manager.dialogs.len() > 2 {
            window_manager.dialogs[2].toggle();
        }
        
        // ESC 退出
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 处理鼠标点击 - 窗口置顶
        if is_mouse_button_pressed(MouseButton::Left) {
            let mouse_pos = mouse_position();
            window_manager.handle_click(vec2(mouse_pos.0, mouse_pos.1));
        }
        
        // 更新和绘制所有窗口
        window_manager.update_and_draw();
        
        // 绘制 FPS
        draw_text(
            &format!("FPS: {}", get_fps()),
            screen_width() - 100.0, 30.0, 20.0, GREEN
        );
        
        next_frame().await;
    }
    
    println!("👋 程序退出");
}
