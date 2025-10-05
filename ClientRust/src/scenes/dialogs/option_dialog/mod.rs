/// OptionDialog - 游戏设置对话框
///
/// 提供游戏选项设置，包括技能模式、特效、音量等
///
/// # 功能特性
/// - 技能模式切换（Tilde/Ctrl）
/// - 技能栏显示开关
/// - 特效开关
/// - 掉落物显示开关
/// - 名字显示开关
/// - HP/MP显示模式
/// - 音量控制（音效+音乐）
/// - 移动模式（新/旧）
/// - 观察模式开关

use std::collections::HashMap;

/// 选项类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionType {
    SkillMode,      // 技能模式（Tilde=false, Ctrl=true）
    SkillBar,       // 技能栏显示
    Effect,         // 特效
    DropView,       // 掉落物显示
    NameView,       // 名字显示
    HPView,         // HP/MP显示模式
    NewMove,        // 新移动模式
    Observe,        // 观察模式
}

/// 游戏设置对话框
pub struct OptionDialog {
    /// 是否可见
    pub visible: bool,

    /// 对话框位置（默认居中）
    pub position: (i32, i32),

    /// 对话框大小 (从 Index 411 推断约 400x350)
    pub size: (i32, i32),

    /// 是否可移动
    pub movable: bool,

    /// 是否排序（Z-order）
    pub sort: bool,

    /// 选项值（true/false）
    pub options: HashMap<OptionType, bool>,

    /// 音量 (0-100)
    pub sound_volume: u8,

    /// 音乐音量 (0-100)
    pub music_volume: u8,

    /// 音量条是否正在拖动
    pub sound_bar_dragging: bool,

    /// 音乐音量条是否正在拖动
    pub music_bar_dragging: bool,
}

impl OptionDialog {
    /// 创建新的设置对话框
    ///
    /// # Arguments
    /// * `screen_width` - 屏幕宽度
    /// * `screen_height` - 屏幕高度
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        let size = (400, 350);
        let position = ((screen_width - size.0) / 2, (screen_height - size.1) / 2);

        // 初始化默认选项
        let mut options = HashMap::new();
        options.insert(OptionType::SkillMode, false); // false = Tilde mode
        options.insert(OptionType::SkillBar, true);
        options.insert(OptionType::Effect, true);
        options.insert(OptionType::DropView, true);
        options.insert(OptionType::NameView, true);
        options.insert(OptionType::HPView, true);  // true = Mode 1
        options.insert(OptionType::NewMove, true);
        options.insert(OptionType::Observe, false);

        Self {
            visible: false,
            position,
            size,
            movable: true,
            sort: true,
            options,
            sound_volume: 50,
            music_volume: 50,
            sound_bar_dragging: false,
            music_bar_dragging: false,
        }
    }

    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.sound_bar_dragging = false;
        self.music_bar_dragging = false;
    }

    /// 切换可见性
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// 检查是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 获取选项值
    pub fn get_option(&self, option: OptionType) -> bool {
        self.options.get(&option).copied().unwrap_or(false)
    }

    /// 设置选项值
    pub fn set_option(&mut self, option: OptionType, value: bool) {
        self.options.insert(option, value);
    }

    /// 切换选项值
    pub fn toggle_option(&mut self, option: OptionType) {
        let current = self.get_option(option);
        self.set_option(option, !current);
    }

    /// 设置音量 (0-100)
    pub fn set_sound_volume(&mut self, volume: u8) {
        self.sound_volume = volume.min(100);
    }

    /// 设置音乐音量 (0-100)
    pub fn set_music_volume(&mut self, volume: u8) {
        self.music_volume = volume.min(100);
    }

    /// 获取音量百分比字符串
    pub fn get_sound_volume_text(&self) -> String {
        format!("{}%", self.sound_volume)
    }

    /// 获取音乐音量百分比字符串
    pub fn get_music_volume_text(&self) -> String {
        format!("{}%", self.music_volume)
    }

    /// 获取音量条位置 (基于当前音量)
    pub fn get_sound_bar_position(&self) -> (i32, i32) {
        let bar_width = 78; // 音量条宽度
        let percent = self.sound_volume as f32 / 100.0;
        let x = if percent > 0.0 {
            159 + ((bar_width - 2) as f32 * percent) as i32
        } else {
            159
        };
        (x, 218)
    }

    /// 获取音乐音量条位置
    pub fn get_music_bar_position(&self) -> (i32, i32) {
        let bar_width = 78;
        let percent = self.music_volume as f32 / 100.0;
        let x = if percent > 0.0 {
            159 + ((bar_width - 2) as f32 * percent) as i32
        } else {
            159
        };
        (x, 244)
    }

    /// 处理音量条拖动
    pub fn on_sound_bar_drag(&mut self, mouse_x: i32) {
        if !self.sound_bar_dragging {
            return;
        }

        // 音量条区域：x=159, width=78
        let bar_x = self.position.0 + 159;
        let bar_width = 78;

        let relative_x = (mouse_x - bar_x).max(0).min(bar_width);
        let volume = (relative_x as f32 / bar_width as f32 * 100.0) as u8;

        self.set_sound_volume(volume);
    }

    /// 处理音乐音量条拖动
    pub fn on_music_bar_drag(&mut self, mouse_x: i32) {
        if !self.music_bar_dragging {
            return;
        }

        let bar_x = self.position.0 + 159;
        let bar_width = 78;

        let relative_x = (mouse_x - bar_x).max(0).min(bar_width);
        let volume = (relative_x as f32 / bar_width as f32 * 100.0) as u8;

        self.set_music_volume(volume);
    }

    /// 获取按钮位置（基于类型）
    pub fn get_button_position(&self, option: OptionType, is_on: bool) -> (i32, i32) {
        let base_x = if is_on { 159 } else { 201 };

        let y = match option {
            OptionType::SkillMode => 68,
            OptionType::SkillBar => 93,
            OptionType::Effect => 118,
            OptionType::DropView => 143,
            OptionType::NameView => 168,
            OptionType::HPView => 193,
            OptionType::Observe => 271,
            OptionType::NewMove => 296,
        };

        (base_x, y)
    }

    /// 获取按钮图像索引（基于选项值）
    pub fn get_button_index(&self, option: OptionType, is_on: bool) -> i32 {
        let value = self.get_option(option);

        match option {
            OptionType::SkillMode => {
                if is_on {
                    if value { 452 } else { 450 }
                } else {
                    if value { 455 } else { 453 }
                }
            }
            OptionType::HPView => {
                if is_on {
                    if value { 464 } else { 462 }
                } else {
                    if value { 467 } else { 465 }
                }
            }
            OptionType::NewMove => {
                if is_on {
                    if value { 853 } else { 851 }
                } else {
                    if value { 848 } else { 850 }
                }
            }
            _ => {
                // 通用按钮索引
                if is_on {
                    if value { 458 } else { 456 }
                } else {
                    if value { 461 } else { 459 }
                }
            }
        }
    }

    /// 鼠标点击事件处理
    ///
    /// # Returns
    /// (选项类型, 点击的是ON还是OFF按钮)
    pub fn on_mouse_click(&mut self, x: i32, y: i32) -> Option<(OptionType, bool)> {
        if !self.visible {
            return None;
        }

        // 检查关闭按钮 (Size.Width - 26, 5)
        let close_x = self.position.0 + self.size.0 - 26;
        let close_y = self.position.1 + 5;
        if x >= close_x && x < close_x + 24 && y >= close_y && y < close_y + 24 {
            self.hide();
            return None;
        }

        // 检查所有选项按钮
        for &option in &[
            OptionType::SkillMode,
            OptionType::SkillBar,
            OptionType::Effect,
            OptionType::DropView,
            OptionType::NameView,
            OptionType::HPView,
            OptionType::Observe,
            OptionType::NewMove,
        ] {
            // ON 按钮 (159, y, 36x17)
            let (on_x, on_y) = self.get_button_position(option, true);
            let on_x = self.position.0 + on_x;
            let on_y = self.position.1 + on_y;
            if x >= on_x && x < on_x + 36 && y >= on_y && y < on_y + 17 {
                self.set_option(option, true);
                return Some((option, true));
            }

            // OFF 按钮 (201, y, 36x17)
            let (off_x, off_y) = self.get_button_position(option, false);
            let off_x = self.position.0 + off_x;
            let off_y = self.position.1 + off_y;
            if x >= off_x && x < off_x + 36 && y >= off_y && y < off_y + 17 {
                self.set_option(option, false);
                return Some((option, false));
            }
        }

        // 检查音量条点击
        let sound_bar_x = self.position.0 + 159;
        let sound_bar_y = self.position.1 + 225;
        if x >= sound_bar_x && x < sound_bar_x + 78 && y >= sound_bar_y && y < sound_bar_y + 20 {
            self.sound_bar_dragging = true;
            self.on_sound_bar_drag(x);
            return None;
        }

        let music_bar_x = self.position.0 + 159;
        let music_bar_y = self.position.1 + 251;
        if x >= music_bar_x && x < music_bar_x + 78 && y >= music_bar_y && y < music_bar_y + 20 {
            self.music_bar_dragging = true;
            self.on_music_bar_drag(x);
            return None;
        }

        None
    }

    /// 鼠标释放事件
    pub fn on_mouse_release(&mut self) {
        self.sound_bar_dragging = false;
        self.music_bar_dragging = false;
    }

    /// 鼠标移动事件
    pub fn on_mouse_move(&mut self, x: i32, _y: i32) {
        if self.sound_bar_dragging {
            self.on_sound_bar_drag(x);
        }
        if self.music_bar_dragging {
            self.on_music_bar_drag(x);
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_dialog_creation() {
        let dialog = OptionDialog::new(1024, 768);

        assert!(!dialog.visible);
        assert!(dialog.movable);
        assert_eq!(dialog.sound_volume, 50);
        assert_eq!(dialog.music_volume, 50);
    }

    #[test]
    fn test_options_default() {
        let dialog = OptionDialog::new(1024, 768);

        assert!(!dialog.get_option(OptionType::SkillMode)); // Tilde mode
        assert!(dialog.get_option(OptionType::SkillBar));
        assert!(dialog.get_option(OptionType::Effect));
        assert!(!dialog.get_option(OptionType::Observe));
    }

    #[test]
    fn test_toggle_option() {
        let mut dialog = OptionDialog::new(1024, 768);

        assert!(dialog.get_option(OptionType::Effect));
        dialog.toggle_option(OptionType::Effect);
        assert!(!dialog.get_option(OptionType::Effect));
        dialog.toggle_option(OptionType::Effect);
        assert!(dialog.get_option(OptionType::Effect));
    }

    #[test]
    fn test_volume_control() {
        let mut dialog = OptionDialog::new(1024, 768);

        dialog.set_sound_volume(75);
        assert_eq!(dialog.sound_volume, 75);
        assert_eq!(dialog.get_sound_volume_text(), "75%");

        dialog.set_music_volume(30);
        assert_eq!(dialog.music_volume, 30);
        assert_eq!(dialog.get_music_volume_text(), "30%");

        // 测试最大值限制
        dialog.set_sound_volume(150);
        assert_eq!(dialog.sound_volume, 100);
    }

    #[test]
    fn test_volume_bar_position() {
        let mut dialog = OptionDialog::new(1024, 768);

        dialog.set_sound_volume(0);
        let pos = dialog.get_sound_bar_position();
        assert_eq!(pos.0, 159); // 起始位置

        dialog.set_sound_volume(100);
        let pos = dialog.get_sound_bar_position();
        assert!(pos.0 > 159); // 应该向右移动
    }

    #[test]
    fn test_button_positions() {
        let dialog = OptionDialog::new(1024, 768);

        let (on_x, on_y) = dialog.get_button_position(OptionType::SkillMode, true);
        assert_eq!(on_x, 159);
        assert_eq!(on_y, 68);

        let (off_x, off_y) = dialog.get_button_position(OptionType::SkillMode, false);
        assert_eq!(off_x, 201);
        assert_eq!(off_y, 68);
    }

    #[test]
    fn test_button_indices() {
        let mut dialog = OptionDialog::new(1024, 768);

        // SkillMode OFF -> ON button应该是450
        dialog.set_option(OptionType::SkillMode, false);
        assert_eq!(dialog.get_button_index(OptionType::SkillMode, true), 450);

        // SkillMode ON -> ON button应该是452
        dialog.set_option(OptionType::SkillMode, true);
        assert_eq!(dialog.get_button_index(OptionType::SkillMode, true), 452);
    }

    #[test]
    fn test_show_hide() {
        let mut dialog = OptionDialog::new(1024, 768);

        dialog.show();
        assert!(dialog.is_visible());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.sound_bar_dragging);
        assert!(!dialog.music_bar_dragging);
    }

    #[test]
    fn test_volume_bar_dragging() {
        let mut dialog = OptionDialog::new(1024, 768);
        dialog.position = (300, 200);

        // 开始拖动音量条
        dialog.sound_bar_dragging = true;
        dialog.on_sound_bar_drag(300 + 159 + 39); // 拖动到中间（50%）

        // 音量应该约为50%
        assert!(dialog.sound_volume >= 48 && dialog.sound_volume <= 52);
    }

    #[test]
    fn test_mouse_release() {
        let mut dialog = OptionDialog::new(1024, 768);

        dialog.sound_bar_dragging = true;
        dialog.music_bar_dragging = true;

        dialog.on_mouse_release();

        assert!(!dialog.sound_bar_dragging);
        assert!(!dialog.music_bar_dragging);
    }
}