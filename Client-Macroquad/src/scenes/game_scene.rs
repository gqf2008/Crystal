// GameScene - 游戏主场景
// 
// 快捷键:
//   I = 背包, C = 角色, B = 快捷栏, S = 商城
//   F1-F6 = 快捷栏技能
//   ESC = 返回角色选择

use crate::game::GameResult;
use crate::scenes::dialogs::game::{
    InventoryDialogHybrid,
    CharacterDialogHybrid,
    BeltDialogHybrid,
    GameShopDialog,
    ChatDialogHybrid,
};
use crate::scenes::{Scene, SceneTransition};
use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;

/// 游戏主场景 - 集成所有混合对话框
pub struct GameScene {
    // UI 对话框
    inventory_dialog: InventoryDialogHybrid,
    character_dialog: CharacterDialogHybrid,
    belt_dialog: BeltDialogHybrid,
    shop_dialog: GameShopDialog,
    chat_dialog: ChatDialogHybrid,
    
    // 初始化状态
    initialized: bool,
}

impl GameScene {
    pub fn new() -> Self {
        let sh = screen_height();
        Self {
            inventory_dialog: InventoryDialogHybrid::new(),
            character_dialog: CharacterDialogHybrid::new(),
            belt_dialog: BeltDialogHybrid::new(),
            shop_dialog: GameShopDialog::new(),
            chat_dialog: ChatDialogHybrid::new(0.0, sh, 0), // 初始位置，后续在 load_textures 中调整
            initialized: false,
        }
    }
    
    /// 异步加载所有对话框纹理
    pub async fn load_textures(&mut self) {
        println!("🎮 GameScene: 加载对话框纹理...");
        
        self.inventory_dialog.load_textures().await;
        self.character_dialog.load_textures().await;
        self.belt_dialog.load_textures().await;
        self.shop_dialog.load_textures().await;
        self.chat_dialog.load_textures().await;
        
        // 设置初始位置
        let sw = screen_width();
        let sh = screen_height();
        
        // 背包在右侧
        self.inventory_dialog.set_position(vec2(sw - 280.0, 100.0));
        
        // 角色在左侧
        self.character_dialog.set_position(vec2(50.0, 100.0));
        
        // 快捷栏在底部中间
        self.belt_dialog.set_position(vec2((sw - 230.0) / 2.0, sh - 60.0));
        self.belt_dialog.open(); // 快捷栏默认打开
        
        // 商城居中
        self.shop_dialog.set_position(vec2((sw - 720.0) / 2.0, 100.0));
        
        // 聊天窗口在左下角
        self.chat_dialog.set_position(vec2(0.0, sh - 100.0));
        self.chat_dialog.open(); // 聊天窗口默认打开
        
        // 添加欢迎消息
        self.chat_dialog.add_message("欢迎进入传奇世界！", Color::from_rgba(255, 255, 0, 255));
        self.chat_dialog.add_message("按 Enter 打开聊天输入框", Color::from_rgba(200, 200, 200, 255));
        
        self.initialized = true;
        println!("✅ GameScene: 对话框纹理加载完成");
    }
    
    /// 处理快捷键
    fn handle_hotkeys(&mut self) {
        // 如果聊天输入框激活，不处理其他快捷键
        if self.chat_dialog.is_input_active() {
            return;
        }
        
        // Enter = 激活聊天输入框
        if is_key_pressed(KeyCode::Enter) {
            self.chat_dialog.activate_input();
            println!("💬 聊天: 输入框已激活");
        }
        
        // I = 背包
        if is_key_pressed(KeyCode::I) {
            self.inventory_dialog.toggle();
            println!("📦 背包: {}", if self.inventory_dialog.is_visible() { "打开" } else { "关闭" });
        }
        
        // C = 角色
        if is_key_pressed(KeyCode::C) {
            self.character_dialog.toggle();
            println!("👤 角色: {}", if self.character_dialog.is_visible() { "打开" } else { "关闭" });
        }
        
        // B = 快捷栏
        if is_key_pressed(KeyCode::B) {
            self.belt_dialog.toggle();
            println!("🎒 快捷栏: {}", if self.belt_dialog.is_visible() { "打开" } else { "关闭" });
        }
        
        // S = 商城
        if is_key_pressed(KeyCode::S) {
            self.shop_dialog.toggle();
            println!("🛒 商城: {}", if self.shop_dialog.is_visible() { "打开" } else { "关闭" });
        }
        
        // ESC = 关闭所有对话框 或 返回角色选择
        // (在 update 中处理返回逻辑)
        if is_key_pressed(KeyCode::Escape) {
            let any_visible = self.inventory_dialog.is_visible() 
                || self.character_dialog.is_visible()
                || self.shop_dialog.is_visible();
            
            if any_visible {
                self.inventory_dialog.close();
                self.character_dialog.close();
                self.shop_dialog.close();
                println!("❌ 关闭所有对话框");
            }
        }
        
        // 1-4 切换角色标签页
        if self.character_dialog.is_visible() {
            if is_key_pressed(KeyCode::Key1) {
                self.character_dialog.switch_tab(crate::scenes::dialogs::game::CharacterTabHybrid::Character);
            }
            if is_key_pressed(KeyCode::Key2) {
                self.character_dialog.switch_tab(crate::scenes::dialogs::game::CharacterTabHybrid::Status);
            }
            if is_key_pressed(KeyCode::Key3) {
                self.character_dialog.switch_tab(crate::scenes::dialogs::game::CharacterTabHybrid::State);
            }
            if is_key_pressed(KeyCode::Key4) {
                self.character_dialog.switch_tab(crate::scenes::dialogs::game::CharacterTabHybrid::Skills);
            }
        }
    }
    
    /// 绘制快捷键提示
    fn draw_help_text(&self) {
        let y = screen_height() - 25.0;
        draw_text_cn(
            "快捷键: I=背包 C=角色 B=快捷栏 S=商城 Enter=聊天 | ESC=返回角色选择",
            10.0, y, 14.0, Color::from_rgba(200, 200, 200, 180)
        );
    }
}

impl Default for GameScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for GameScene {
    fn name(&self) -> &str { "游戏场景" }
    
    fn on_enter(&mut self) -> GameResult {
        println!("🎬 进入游戏场景");
        // 注意: 纹理需要异步加载，这里无法调用 async 函数
        // 应该在进入场景前或通过 Loading 场景预加载
        Ok(())
    }
    
    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开游戏场景");
        // 关闭所有对话框
        self.inventory_dialog.close();
        self.character_dialog.close();
        self.belt_dialog.close();
        self.shop_dialog.close();
        self.chat_dialog.close();
        self.chat_dialog.deactivate_input(); // 确保 IME 被禁用
        Ok(())
    }
    
    fn update(&mut self, _dt: f32) -> GameResult<SceneTransition> {
        // 处理快捷键
        self.handle_hotkeys();
        
        // ESC 且没有打开的对话框 = 返回角色选择
        if is_key_pressed(KeyCode::Escape) {
            let any_visible = self.inventory_dialog.is_visible() 
                || self.character_dialog.is_visible()
                || self.shop_dialog.is_visible();
            
            if !any_visible {
                return Ok(SceneTransition::CharacterSelect);
            }
        }
        
        Ok(SceneTransition::None)
    }
    
    fn render(&mut self) -> GameResult {
        // 深绿色背景（游戏地图区域）
        clear_background(Color::from_rgba(30, 45, 30, 255));
        
        // 绘制网格背景（模拟地图）
        let grid_color = Color::from_rgba(50, 65, 50, 255);
        for i in 0..=((screen_width() / 48.0) as i32 + 1) {
            let x = i as f32 * 48.0;
            draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
        }
        for i in 0..=((screen_height() / 32.0) as i32 + 1) {
            let y = i as f32 * 32.0;
            draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
        }
        
        // 提示文字
        if !self.initialized {
            draw_text_cn(
                "⏳ 正在加载游戏资源...",
                screen_width() / 2.0 - 100.0,
                screen_height() / 2.0,
                24.0, WHITE
            );
        } else {
            // 绘制所有对话框（商城在最上层）
            self.chat_dialog.update_and_draw(); // 聊天窗口在最底层
            self.inventory_dialog.update_and_draw();
            self.character_dialog.update_and_draw();
            self.belt_dialog.update_and_draw();
            self.shop_dialog.update_and_draw();
            
            // 绘制帮助提示
            self.draw_help_text();
        }
        
        Ok(())
    }
    
    fn handle_input(&mut self) -> GameResult { 
        Ok(()) 
    }
}
