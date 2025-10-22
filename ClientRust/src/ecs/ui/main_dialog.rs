// ============================================================================
// GameScene UI 组件 - 游戏主界面
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, Text, DrawParam, Rect, Mesh, DrawMode, PxScale};
use crate::graphics::libraries::{get_library, LibraryName};
use crate::ecs::ui::{ButtonWidget, ButtonGroup};

/// 主界面对话框 - 显示生命/魔法条、技能栏、快捷键等
/// 
/// Mirrors C# MainDialog:
/// ```csharp
/// public sealed class MainDialog : MirImageControl
/// {
///     Index = Settings.Resolution == 800 ? 0 : Settings.Resolution == 1024 ? 1 : 2;
///     Library = Libraries.Prguse;
///     Location = new Point(((Settings.ScreenWidth / 2) - (Size.Width / 2)), ScreenHeight - Size.Height);
/// }
/// ```
pub struct MainDialog {
    /// 屏幕宽度
    screen_width: f32,
    
    /// 屏幕高度  
    screen_height: f32,
    
    /// 主界面背景索引 (根据分辨率: 800->0, 1024->1, 1280+->2)
    bg_index: i32,
    
    /// 底部按钮组
    buttons: ButtonGroup,
    
    /// 生命值 (0-100)
    pub health: f32,
    
    /// 魔法值 (0-100)
    pub mana: f32,
    
    /// 经验值百分比 (0-100)
    pub experience: f32,
    
    /// 等级
    pub level: i32,
    
    /// 角色名称
    pub character_name: String,
    
    /// 金币
    pub gold: u32,
    
    /// 当前负重/最大负重
    pub weight: (u32, u32),
    
    /// 背包空间 (已用/总计)
    pub bag_space: (u32, u32),
}

impl MainDialog {
    /// 创建主界面对话框
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        // 根据分辨率选择背景索引
        let bg_index = if screen_width <= 800.0 {
            0  // Prguse_0 (800x600)
        } else if screen_width <= 1024.0 {
            1  // Prguse_1 (1024x768)
        } else {
            2  // Prguse_2 (1280x1024+)
        };
        
        // 创建底部按钮组
        let mut buttons = ButtonGroup::new();
        
        // 计算按钮位置 (相对于主界面底部)
        let dialog_width = if screen_width <= 800.0 { 800.0 } else if screen_width <= 1024.0 { 1024.0 } else { 1280.0 };
        let dialog_x = (screen_width - dialog_width) / 2.0;
        let button_y = screen_height - 168.0 + 76.0;  // MainDialog 高度约 168, 按钮在 Y=76
        
        // 背包按钮 (Inventory) - Prguse_1903/1904/1905
        buttons.add(
            ButtonWidget::new(1, dialog_x + dialog_width - 96.0, button_y, 23.0, 24.0, 1903)
                .with_tooltip("背包 (I)")
        );
        
        // 角色按钮 (Character) - Prguse_1900/1901/1902
        buttons.add(
            ButtonWidget::new(2, dialog_x + dialog_width - 119.0, button_y, 23.0, 24.0, 1900)
                .with_tooltip("角色 (C)")
        );
        
        // 技能按钮 (Skills) - Prguse_1906/1907/1908
        buttons.add(
            ButtonWidget::new(3, dialog_x + dialog_width - 73.0, button_y, 23.0, 24.0, 1906)
                .with_tooltip("技能 (S)")
        );
        
        // 任务按钮 (Quest) - Prguse_1909/1910/1911
        buttons.add(
            ButtonWidget::new(4, dialog_x + dialog_width - 50.0, button_y, 23.0, 24.0, 1909)
                .with_tooltip("任务 (Q)")
        );
        
        // 选项按钮 (Options) - Prguse_1912/1913/1914
        buttons.add(
            ButtonWidget::new(5, dialog_x + dialog_width - 27.0, button_y, 23.0, 24.0, 1912)
                .with_tooltip("选项 (O)")
        );
        
        // 菜单按钮 (Menu) - Prguse_1960/1961/1962
        buttons.add(
            ButtonWidget::new(6, dialog_x + dialog_width - 55.0, button_y - 41.0, 50.0, 27.0, 1960)
                .with_tooltip("菜单")
        );
        
        // 商城按钮 (GameShop) - Prguse_826/827/828
        buttons.add(
            ButtonWidget::new(7, dialog_x + dialog_width - 105.0, button_y - 41.0, 50.0, 27.0, 826)
                .with_tooltip("商城")
        );
        
        Self {
            screen_width,
            screen_height,
            bg_index,
            buttons,
            health: 100.0,
            mana: 100.0,
            experience: 0.0,
            level: 1,
            character_name: "勇士".to_string(),
            gold: 0,
            weight: (0, 100),
            bag_space: (0, 40),
        }
    }
    
    /// 更新屏幕尺寸
    pub fn resize(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
        
        // 重新计算按钮位置
        let dialog_width = if width <= 800.0 { 800.0 } else if width <= 1024.0 { 1024.0 } else { 1280.0 };
        let dialog_x = (width - dialog_width) / 2.0;
        let button_y = height - 168.0 + 76.0;
        
        // 更新按钮位置
        if let Some(btn) = self.buttons.get_mut(1) {
            btn.x = dialog_x + dialog_width - 96.0;
            btn.y = button_y;
        }
        if let Some(btn) = self.buttons.get_mut(2) {
            btn.x = dialog_x + dialog_width - 119.0;
            btn.y = button_y;
        }
        if let Some(btn) = self.buttons.get_mut(3) {
            btn.x = dialog_x + dialog_width - 73.0;
            btn.y = button_y;
        }
        if let Some(btn) = self.buttons.get_mut(4) {
            btn.x = dialog_x + dialog_width - 50.0;
            btn.y = button_y;
        }
        if let Some(btn) = self.buttons.get_mut(5) {
            btn.x = dialog_x + dialog_width - 27.0;
            btn.y = button_y;
        }
        if let Some(btn) = self.buttons.get_mut(6) {
            btn.x = dialog_x + dialog_width - 55.0;
            btn.y = button_y - 41.0;
        }
        if let Some(btn) = self.buttons.get_mut(7) {
            btn.x = dialog_x + dialog_width - 105.0;
            btn.y = button_y - 41.0;
        }
    }
    
    /// 更新鼠标悬停状态
    pub fn update_hover(&mut self, mouse_x: f32, mouse_y: f32) {
        self.buttons.update_hover(mouse_x, mouse_y);
    }
    
    /// 处理鼠标点击
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> Option<MainDialogButton> {
        if let Some(button_id) = self.buttons.on_mouse_down(x, y) {
            match button_id {
                1 => Some(MainDialogButton::Inventory),
                2 => Some(MainDialogButton::Character),
                3 => Some(MainDialogButton::Skills),
                4 => Some(MainDialogButton::Quest),
                5 => Some(MainDialogButton::Options),
                6 => Some(MainDialogButton::Menu),
                7 => Some(MainDialogButton::GameShop),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// 绘制主界面
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let dialog_width = if self.screen_width <= 800.0 { 800.0 } else if self.screen_width <= 1024.0 { 1024.0 } else { 1280.0 };
        let dialog_x = (self.screen_width - dialog_width) / 2.0;
        let dialog_y = self.screen_height - 168.0;
        
        // 1. 绘制主界面背景 (Prguse_0/1/2)
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(
                    ctx, canvas,
                    self.bg_index as usize,
                    dialog_x, dialog_y,
                    Color::WHITE, false
                );
            }
        }
        
        // 2. 绘制生命球 (红色进度条)
        self.draw_health_orb(ctx, canvas, dialog_x + 9.0, dialog_y + 30.0)?;
        
        // 3. 绘制魔法球 (蓝色进度条,如果不是纯战士)
        if self.level >= 26 {
            self.draw_mana_orb(ctx, canvas, dialog_x + 9.0, dialog_y + 30.0)?;
        }
        
        // 4. 绘制经验条
        self.draw_experience_bar(ctx, canvas, dialog_x + 9.0, dialog_y + 143.0)?;
        
        // 5. 绘制负重条
        self.draw_weight_bar(ctx, canvas, dialog_x + dialog_width - 105.0, dialog_y + 103.0)?;
        
        // 6. 绘制文字信息
        self.draw_labels(ctx, canvas, dialog_x, dialog_y)?;
        
        // 7. 绘制按钮
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                for button in &self.buttons.buttons {
                    let texture_index = button.get_texture_index();
                    let color = button.get_color();
                    let _ = lib.draw_with_color(
                        ctx, canvas,
                        texture_index as usize,
                        button.x, button.y,
                        color, false
                    );
                }
            }
        }
        
        // 8. 绘制工具提示
        for button in &self.buttons.buttons {
            if let Some(tooltip_text) = button.get_tooltip() {
                self.draw_tooltip(ctx, canvas, tooltip_text, button.x, button.y - 25.0)?;
                break;
            }
        }
        
        Ok(())
    }
    
    /// 绘制生命球
    fn draw_health_orb(&self, ctx: &mut Context, canvas: &mut Canvas, x: f32, y: f32) -> GameResult {
        let health_percent = self.health / 100.0;
        let bar_width = 80.0 * health_percent;
        
        // 绘制红色生命条
        if bar_width > 0.0 {
            let rect = Rect::new(x + 3.0, y + 7.0, bar_width, 12.0);
            if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), rect, Color::from_rgb(200, 0, 0)) {
                canvas.draw(&mesh, DrawParam::default());
            }
        }
        
        // 绘制生命值文字
        let hp_text = format!("{:.0}", self.health);
        let mut text = Text::new(&hp_text);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(PxScale::from(12.0));
        canvas.draw(&text, DrawParam::default()
            .dest([x + 42.0 - text.measure(ctx)?.x / 2.0, y + 7.0])
            .color(Color::WHITE));
        
        Ok(())
    }
    
    /// 绘制魔法球
    fn draw_mana_orb(&self, ctx: &mut Context, canvas: &mut Canvas, x: f32, y: f32) -> GameResult {
        let mana_percent = self.mana / 100.0;
        let bar_width = 80.0 * mana_percent;
        
        // 绘制蓝色魔法条
        if bar_width > 0.0 {
            let rect = Rect::new(x + 3.0, y + 22.0, bar_width, 12.0);
            if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), rect, Color::from_rgb(0, 0, 200)) {
                canvas.draw(&mesh, DrawParam::default());
            }
        }
        
        // 绘制魔法值文字
        let mp_text = format!("{:.0}", self.mana);
        let mut text = Text::new(&mp_text);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(PxScale::from(12.0));
        canvas.draw(&text, DrawParam::default()
            .dest([x + 42.0 - text.measure(ctx)?.x / 2.0, y + 22.0])
            .color(Color::WHITE));
        
        Ok(())
    }
    
    /// 绘制经验条
    fn draw_experience_bar(&self, ctx: &mut Context, canvas: &mut Canvas, x: f32, y: f32) -> GameResult {
        let exp_percent = self.experience / 100.0;
        let bar_width = 85.0 * exp_percent;
        
        // 绘制黄色经验条
        if bar_width > 0.0 {
            let rect = Rect::new(x, y, bar_width, 14.0);
            if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), rect, Color::from_rgb(200, 200, 0)) {
                canvas.draw(&mesh, DrawParam::default());
            }
        }
        
        Ok(())
    }
    
    /// 绘制负重条
    fn draw_weight_bar(&self, ctx: &mut Context, canvas: &mut Canvas, x: f32, y: f32) -> GameResult {
        let weight_percent = self.weight.0 as f32 / self.weight.1 as f32;
        let bar_width = 90.0 * weight_percent;
        
        // 根据负重百分比改变颜色
        let color = if weight_percent > 0.9 {
            Color::from_rgb(200, 0, 0)  // 红色 - 超重
        } else if weight_percent > 0.7 {
            Color::from_rgb(200, 200, 0)  // 黄色 - 接近超重
        } else {
            Color::from_rgb(0, 200, 0)  // 绿色 - 正常
        };
        
        if bar_width > 0.0 {
            let rect = Rect::new(x, y, bar_width, 12.0);
            if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), rect, color) {
                canvas.draw(&mesh, DrawParam::default());
            }
        }
        
        Ok(())
    }
    
    /// 绘制标签文字
    fn draw_labels(&self, ctx: &mut Context, canvas: &mut Canvas, dialog_x: f32, dialog_y: f32) -> GameResult {
        // 等级标签 (左上角)
        let level_text = format!("Lv.{}", self.level);
        let mut text = Text::new(&level_text);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(PxScale::from(12.0));
        canvas.draw(&text, DrawParam::default()
            .dest([dialog_x + 5.0, dialog_y + 108.0])
            .color(Color::from_rgb(255, 255, 200)));
        
        // 角色名称 (居中)
        let mut name_text = Text::new(&self.character_name);
        name_text.set_font("AlibabaPuHuiTi");
        name_text.set_scale(PxScale::from(14.0));
        let name_width = name_text.measure(ctx)?.x;
        canvas.draw(&name_text, DrawParam::default()
            .dest([dialog_x + 6.0 + (90.0 - name_width) / 2.0, dialog_y + 120.0])
            .color(Color::WHITE));
        
        // 金币 (右侧)
        let dialog_width = if self.screen_width <= 800.0 { 800.0 } else if self.screen_width <= 1024.0 { 1024.0 } else { 1280.0 };
        let gold_text = format!("{}", self.gold);
        let mut text = Text::new(&gold_text);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(PxScale::from(10.0));
        canvas.draw(&text, DrawParam::default()
            .dest([dialog_x + dialog_width - 105.0, dialog_y + 119.0])
            .color(Color::from_rgb(255, 215, 0)));
        
        // 负重 (右侧)
        let weight_text = format!("{}/{}", self.weight.0, self.weight.1);
        let mut text = Text::new(&weight_text);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(PxScale::from(10.0));
        canvas.draw(&text, DrawParam::default()
            .dest([dialog_x + dialog_width - 105.0, dialog_y + 101.0])
            .color(Color::WHITE));
        
        // 背包空间 (右侧)
        let space_text = format!("{}/{}", self.bag_space.0, self.bag_space.1);
        let mut text = Text::new(&space_text);
        text.set_font("AlibabaPuHuiTi");
        text.set_scale(PxScale::from(10.0));
        canvas.draw(&text, DrawParam::default()
            .dest([dialog_x + dialog_width - 30.0, dialog_y + 101.0])
            .color(Color::WHITE));
        
        Ok(())
    }
    
    /// 绘制工具提示
    fn draw_tooltip(&self, ctx: &mut Context, canvas: &mut Canvas, text: &str, x: f32, y: f32) -> GameResult {
        let mut tooltip = Text::new(text);
        tooltip.set_font("AlibabaPuHuiTi");
        tooltip.set_scale(14.0);
        
        let text_bounds = tooltip.measure(ctx).unwrap_or(ggez::glam::Vec2::new(100.0, 20.0).into());
        let bg_rect = Rect::new(
            x - 5.0,
            y - 5.0,
            text_bounds.x + 10.0,
            text_bounds.y + 10.0
        );
        
        // 半透明黑色背景
        if let Ok(mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), bg_rect, Color::from_rgba(0, 0, 0, 200)) {
            canvas.draw(&mesh, DrawParam::default());
        }
        
        // 黄色文字
        canvas.draw(&tooltip, DrawParam::default()
            .dest([x, y])
            .color(Color::from_rgb(255, 255, 200)));
        
        Ok(())
    }
}

/// 主界面按钮类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainDialogButton {
    Inventory,   // 背包
    Character,   // 角色
    Skills,      // 技能
    Quest,       // 任务
    Options,     // 选项
    Menu,        // 菜单
    GameShop,    // 商城
}
