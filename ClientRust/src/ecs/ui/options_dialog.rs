// ============================================================================
// 选项对话框 - OptionsDialog
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 选项标签页
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsTab {
    Game,      // 游戏设置
    Graphics,  // 图形设置
    Audio,     // 音频设置
    Controls,  // 控制设置
}

/// 选项对话框
pub struct OptionsDialog {
    /// 背景图像索引 (Prguse2)
    background_index: u16,
    
    /// 对话框位置 (屏幕坐标)
    position: (f32, f32),
    
    /// 对话框尺寸
    size: (f32, f32),
    
    /// 当前选中的标签页
    current_tab: OptionsTab,
    
    /// 临时设置 (未保存前的状态)
    temp_settings: TempSettings,
}

/// 临时设置
#[derive(Debug, Clone)]
pub struct TempSettings {
    // 游戏设置
    pub show_monster_hp: bool,
    pub show_player_name: bool,
    pub show_monster_name: bool,
    pub show_damage_numbers: bool,
    
    // 图形设置
    pub fullscreen: bool,
    pub resolution: (u32, u32),
    pub vsync: bool,
    pub max_fps: u32,
    
    // 音频设置
    pub master_volume: f32,  // 0.0 - 1.0
    pub music_volume: f32,
    pub sound_volume: f32,
    pub mute_all: bool,
    
    // 控制设置
    pub mouse_sensitivity: f32,
    pub invert_y_axis: bool,
}

impl Default for TempSettings {
    fn default() -> Self {
        Self {
            show_monster_hp: true,
            show_player_name: true,
            show_monster_name: true,
            show_damage_numbers: true,
            
            fullscreen: false,
            resolution: (1024, 768),
            vsync: true,
            max_fps: 60,
            
            master_volume: 0.8,
            music_volume: 0.6,
            sound_volume: 0.8,
            mute_all: false,
            
            mouse_sensitivity: 1.0,
            invert_y_axis: false,
        }
    }
}

impl OptionsDialog {
    /// 创建新的选项对话框
    pub fn new() -> Self {
        Self {
            background_index: 1974, // 选项对话框背景 (需要从 C# 客户端确认)
            position: ((1024.0 - 500.0) / 2.0, (768.0 - 400.0) / 2.0), // 居中
            size: (500.0, 400.0),
            current_tab: OptionsTab::Game,
            temp_settings: TempSettings::default(),
        }
    }
    
    /// 切换标签页
    pub fn switch_tab(&mut self, tab: OptionsTab) {
        self.current_tab = tab;
        tracing::info!("📑 切换到选项标签: {:?}", tab);
    }
    
    /// 保存设置
    pub fn save_settings(&mut self) {
        // TODO: 将临时设置保存到 ClientSettings
        tracing::info!("💾 保存游戏设置");
    }
    
    /// 取消设置 (恢复默认值)
    pub fn cancel_settings(&mut self) {
        self.temp_settings = TempSettings::default();
        tracing::info!("↩️ 取消设置更改");
    }
    
    /// 应用设置 (不关闭对话框)
    pub fn apply_settings(&mut self) {
        // TODO: 应用设置但不关闭对话框
        tracing::info!("✅ 应用游戏设置");
    }
    
    /// 绘制选项对话框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // TODO: 实现实际的选项对话框渲染
        // 1. 绘制背景
        // 2. 绘制标签页按钮
        // 3. 根据当前标签页绘制不同的设置项:
        //    - 游戏设置: 复选框
        //    - 图形设置: 下拉框 + 复选框
        //    - 音频设置: 滑块
        //    - 控制设置: 按键绑定
        // 4. 绘制保存/取消/应用按钮
        
        Ok(())
    }
    
    /// 处理鼠标点击
    pub fn handle_click(&mut self, x: f32, y: f32) -> bool {
        // 检查点击是否在对话框范围内
        if x >= self.position.0 && x <= self.position.0 + self.size.0
            && y >= self.position.1 && y <= self.position.1 + self.size.1
        {
            // TODO: 处理标签页切换、按钮点击、滑块调整等
            return true;
        }
        false
    }
}

/// 选项对话框组件
pub struct OptionsDialogComponent {
    pub dialog: OptionsDialog,
    pub is_open: bool,
}

impl OptionsDialogComponent {
    pub fn new() -> Self {
        Self {
            dialog: OptionsDialog::new(),
            is_open: false,
        }
    }
}
