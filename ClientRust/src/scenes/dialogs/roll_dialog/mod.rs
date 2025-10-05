//! Roll Dialog
//!
//! Dice rolling and Yut game animation dialog.
//! Corresponds to Client/MirScenes/Dialogs/RollDialog.cs

use std::time::{Duration, Instant};

/// 投掷类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollType {
    Die = 0, // 骰子
    Yut = 1, // 韩国传统游戏
}

/// 投掷对话框 - 处理骰子和游戏动画
#[derive(Debug, Clone)]
pub struct RollDialog {
    /// 是否可见
    pub visible: bool,
    /// 窗口位置
    pub location: (i32, i32),
    /// 窗口大小
    pub size: (i32, i32),

    /// 投掷类型
    pub roll_type: RollType,
    /// NPC页面
    pub npc_page: String,
    /// 投掷结果
    pub result: i32,
    /// 当前动画循环
    pub current_loop: i32,
    /// 是否已投掷
    pub rolled: bool,
    /// 是否正在投掷
    pub rolling: bool,
    /// 动画开始时间
    pub animation_start: Option<Instant>,
    /// 动画延迟
    pub animation_delay: Duration,
    /// 动画帧数
    pub animation_count: i32,
}

impl Default for RollDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 300), // Centered on 800x600 screen
            size: (65, 65),
            roll_type: RollType::Die,
            npc_page: String::new(),
            result: 0,
            current_loop: 0,
            rolled: false,
            rolling: false,
            animation_start: None,
            animation_delay: Duration::from_millis(100),
            animation_count: 4,
        }
    }
}

impl RollDialog {
    /// 创建新的投掷对话框
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置投掷参数
    pub fn setup(&mut self, roll_type: RollType, npc_page: String, result: i32, auto_roll: bool) {
        self.roll_type = roll_type;
        self.npc_page = npc_page;
        self.result = result;
        self.rolled = false;
        self.current_loop = 0;
        self.visible = true;
        self.rolling = false;

        match roll_type {
            RollType::Die => {
                self.size = (65, 65);
                self.location = (400 - 38, 300 - 40); // Centered on 800x600
                // _image.Index = 282; _image.Library = Libraries.Prguse;
            }
            RollType::Yut => {
                self.size = (180, 130);
                self.location = (400 - 90, 300 - 65); // Centered on 800x600
                // _image.Index = 2581; _image.Library = Libraries.Items;
            }
        }

        if auto_roll {
            self.roll();
        }
    }

    /// 开始投掷
    pub fn roll(&mut self) {
        self.visible = true;
        self.rolling = true;
        self.animation_start = Some(Instant::now());

        match self.roll_type {
            RollType::Die => {
                // _image.Index = 281 + self.result; _image.Library = Libraries.Prguse;
                self.animation_count = 4;
                self.animation_delay = Duration::from_millis(100);
                // SoundManager.PlaySound(10600);
                println!("Playing die roll sound");
            }
            RollType::Yut => {
                // _image.Index = 2587 + self.result; _image.Library = Libraries.Items;
                self.animation_count = 6;
                self.animation_delay = Duration::from_millis(100);
                // SoundManager.PlaySound(10601);
                println!("Playing yut roll sound");
            }
        }
    }

    /// 处理点击事件
    pub fn handle_click(&mut self) {
        if self.rolling {
            return;
        }

        if self.rolled {
            self.hide();
            return;
        }

        self.roll();
    }

    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
        self.rolled = false;
        self.rolling = false;
        self.animation_start = None;
    }

    /// 处理每帧更新
    pub fn process(&mut self) {
        if !self.rolling || self.animation_start.is_none() {
            return;
        }

        let elapsed = self.animation_start.unwrap().elapsed();

        match self.roll_type {
            RollType::Die => {
                // Die animation: loop 5 times then show result
                let loop_duration = self.animation_delay * self.animation_count as u32;
                let total_loops = 5;

                if elapsed >= loop_duration * total_loops {
                    // Animation finished
                    self.finish_roll();
                } else {
                    // Continue looping
                    self.current_loop = (elapsed.as_millis() / loop_duration.as_millis()) as i32;
                }
            }
            RollType::Yut => {
                // Yut animation: single loop then show result
                let total_duration = self.animation_delay * self.animation_count as u32;

                if elapsed >= total_duration {
                    // Animation finished
                    self.finish_roll();
                }
            }
        }
    }

    /// 完成投掷动画
    fn finish_roll(&mut self) {
        self.rolling = false;
        self.rolled = true;
        // Show result image
        // _image.Visible = true;
        // _animation.Visible = false;
        // _animation.Animated = false;
        self.return_result();
    }

    /// 返回结果给服务器
    fn return_result(&mut self) {
        // TODO: Send result to server
        // if (CMain.Time <= GameScene.NPCTime) return;
        // GameScene.NPCTime = CMain.Time + 5000;
        // Network.Enqueue(new C.CallNPC { ObjectID = GameScene.NPCID, Key = $"[{self.npc_page}]" });
        println!("Returning roll result: {} for page: {}", self.result, self.npc_page);
    }

    /// 获取当前动画帧
    pub fn get_current_frame(&self) -> i32 {
        if !self.rolling || self.animation_start.is_none() {
            return 0;
        }

        let elapsed = self.animation_start.unwrap().elapsed();
        let frame_duration = self.animation_delay;
        let current_frame = (elapsed.as_millis() / frame_duration.as_millis()) as i32 % self.animation_count;

        current_frame
    }

    /// 获取结果图像索引
    pub fn get_result_image_index(&self) -> i32 {
        match self.roll_type {
            RollType::Die => 281 + self.result,
            RollType::Yut => 2587 + self.result,
        }
    }

    /// 获取动画图像索引
    pub fn get_animation_image_index(&self) -> i32 {
        match self.roll_type {
            RollType::Die => 290,
            RollType::Yut => 2581,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_roll_dialog_creation() {
        let dialog = RollDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.roll_type, RollType::Die);
        assert_eq!(dialog.size, (65, 65));
        assert!(!dialog.rolled);
        assert!(!dialog.rolling);
    }

    #[test]
    fn test_setup_die() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Die, "test_page".to_string(), 3, false);

        assert_eq!(dialog.roll_type, RollType::Die);
        assert_eq!(dialog.npc_page, "test_page");
        assert_eq!(dialog.result, 3);
        assert_eq!(dialog.size, (65, 65));
        assert!(dialog.visible);
        assert!(!dialog.rolled);
    }

    #[test]
    fn test_setup_yut() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Yut, "yut_page".to_string(), 2, false);

        assert_eq!(dialog.roll_type, RollType::Yut);
        assert_eq!(dialog.npc_page, "yut_page");
        assert_eq!(dialog.result, 2);
        assert_eq!(dialog.size, (180, 130));
        assert!(dialog.visible);
        assert!(!dialog.rolled);
    }

    #[test]
    fn test_setup_auto_roll() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Die, "auto_page".to_string(), 1, true);

        assert!(dialog.rolling);
        assert!(dialog.animation_start.is_some());
    }

    #[test]
    fn test_roll_die() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Die, "die_page".to_string(), 4, false);

        dialog.roll();

        assert!(dialog.rolling);
        assert!(dialog.animation_start.is_some());
        assert_eq!(dialog.animation_count, 4);
        assert_eq!(dialog.animation_delay, Duration::from_millis(100));
    }

    #[test]
    fn test_roll_yut() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Yut, "yut_page".to_string(), 3, false);

        dialog.roll();

        assert!(dialog.rolling);
        assert!(dialog.animation_start.is_some());
        assert_eq!(dialog.animation_count, 6);
        assert_eq!(dialog.animation_delay, Duration::from_millis(100));
    }

    #[test]
    fn test_handle_click_when_rolling() {
        let mut dialog = RollDialog::new();
        dialog.rolling = true;

        // Should not change state when rolling
        let was_visible = dialog.visible;
        dialog.handle_click();
        assert_eq!(dialog.visible, was_visible);
    }

    #[test]
    fn test_handle_click_when_rolled() {
        let mut dialog = RollDialog::new();
        dialog.rolled = true;
        dialog.visible = true;

        dialog.handle_click();

        assert!(!dialog.visible);
        assert!(!dialog.rolled);
    }

    #[test]
    fn test_handle_click_to_roll() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Die, "click_page".to_string(), 2, false);

        dialog.handle_click();

        assert!(dialog.rolling);
        assert!(dialog.animation_start.is_some());
    }

    #[test]
    fn test_show_hide() {
        let mut dialog = RollDialog::new();

        dialog.show();
        assert!(dialog.visible);

        dialog.hide();
        assert!(!dialog.visible);
        assert!(!dialog.rolled);
        assert!(!dialog.rolling);
    }

    #[test]
    fn test_get_result_image_index_die() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Die, "test".to_string(), 5, false);

        assert_eq!(dialog.get_result_image_index(), 281 + 5);
    }

    #[test]
    fn test_get_result_image_index_yut() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Yut, "test".to_string(), 3, false);

        assert_eq!(dialog.get_result_image_index(), 2587 + 3);
    }

    #[test]
    fn test_get_animation_image_index_die() {
        let mut dialog = RollDialog::new();
        dialog.roll_type = RollType::Die;

        assert_eq!(dialog.get_animation_image_index(), 290);
    }

    #[test]
    fn test_get_animation_image_index_yut() {
        let mut dialog = RollDialog::new();
        dialog.roll_type = RollType::Yut;

        assert_eq!(dialog.get_animation_image_index(), 2581);
    }

    #[test]
    fn test_get_current_frame_no_animation() {
        let dialog = RollDialog::new();
        assert_eq!(dialog.get_current_frame(), 0);
    }

    #[test]
    fn test_process_die_animation() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Die, "test".to_string(), 1, false);
        dialog.roll();

        // Simulate some time passing
        thread::sleep(Duration::from_millis(50));
        dialog.process();

        // Should still be rolling
        assert!(dialog.rolling);
        assert!(dialog.get_current_frame() >= 0);
    }

    #[test]
    fn test_process_yut_animation() {
        let mut dialog = RollDialog::new();
        dialog.setup(RollType::Yut, "test".to_string(), 2, false);
        dialog.roll();

        // Simulate time passing for full animation
        thread::sleep(Duration::from_millis(650)); // 6 frames * 100ms + buffer
        dialog.process();

        // Should have finished
        assert!(!dialog.rolling);
        assert!(dialog.rolled);
    }
}