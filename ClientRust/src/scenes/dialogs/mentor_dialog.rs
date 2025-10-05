//! Mentor Dialog
//!
//! Mentor/mentee relationship management dialog.
//! Corresponds to Client/MirScenes/Dialogs/MentorDialog.cs

use std::time::{Duration, Instant};

/// 导师系统对话框
#[derive(Debug, Clone)]
pub struct MentorDialog {
    /// 是否可见
    pub visible: bool,
    /// 窗口位置
    pub location: (i32, i32),

    /// 导师姓名
    pub mentor_name: String,
    /// 学徒姓名
    pub mentee_name: String,
    /// 导师等级
    pub mentor_level: u16,
    /// 学徒等级
    pub mentee_level: u16,
    /// 导师是否在线
    pub mentor_online: bool,
    /// 学徒是否在线
    pub mentee_online: bool,
    /// 经验值
    pub exp_points: u32,
    /// 是否是导师（true=导师，false=学徒）
    pub is_mentor: bool,
    /// 下次请求时间
    pub next_request_time: Option<Instant>,
}

impl Default for MentorDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 250), // Centered on 800x600 screen
            mentor_name: String::new(),
            mentee_name: String::new(),
            mentor_level: 0,
            mentee_level: 0,
            mentor_online: false,
            mentee_online: false,
            exp_points: 0,
            is_mentor: false,
            next_request_time: None,
        }
    }
}

impl MentorDialog {
    /// 创建新的导师对话框
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示对话框
    pub fn show(&mut self) {
        if self.visible {
            return;
        }
        self.visible = true;
        self.request_mentor_info();
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        if !self.visible {
            return;
        }
        self.visible = false;
    }

    /// 切换显示状态
    pub fn toggle(&mut self) {
        if !self.visible {
            self.show();
        } else {
            self.hide();
        }
    }

    /// 请求导师信息
    pub fn request_mentor_info(&mut self) {
        // TODO: 发送网络请求获取导师信息
        // MirNetwork.Network.Enqueue(new ClientPackets.RequestMentorInfo());
        self.next_request_time = Some(Instant::now() + Duration::from_secs(1));
    }

    /// 接收导师信息
    pub fn receive_mentor_info(&mut self, mentor_name: String, mentee_name: String, mentor_level: u16, mentee_level: u16, mentor_online: bool, mentee_online: bool, exp_points: u32, is_mentor: bool) {
        self.mentor_name = mentor_name;
        self.mentee_name = mentee_name;
        self.mentor_level = mentor_level;
        self.mentee_level = mentee_level;
        self.mentor_online = mentor_online;
        self.mentee_online = mentee_online;
        self.exp_points = exp_points;
        self.is_mentor = is_mentor;
        self.update_interface();
    }

    /// 更新界面显示
    pub fn update_interface(&mut self) {
        // TODO: Update UI elements based on mentor/mentee status
        // This would update labels and button visibility in the actual UI implementation
        if self.is_mentor {
            // 当前用户是导师
            println!("Mentor: {}, Level: {}, Online: {}, Mentee: {}, Level: {}, Online: {}, EXP: {}",
                    self.mentor_name, self.mentor_level, self.mentor_online,
                    self.mentee_name, self.mentee_level, self.mentee_online, self.exp_points);
        } else {
            // 当前用户是学徒
            println!("Mentee: {}, Level: {}, Online: {}, Mentor: {}, Level: {}, Online: {}, EXP: {}",
                    self.mentee_name, self.mentee_level, self.mentee_online,
                    self.mentor_name, self.mentor_level, self.mentor_online, self.exp_points);
        }
    }

    /// 允许导师
    pub fn allow_mentor(&mut self) {
        // TODO: 发送允许导师请求
        // MirNetwork.Network.Enqueue(new ClientPackets.AllowMentor());
        println!("Sending allow mentor request");
    }

    /// 添加导师
    pub fn add_mentor(&mut self, mentor_name: &str) {
        // TODO: 发送添加导师请求
        // MirNetwork.Network.Enqueue(new ClientPackets.AddMentor(mentor_name));
        println!("Sending add mentor request for: {}", mentor_name);
    }

    /// 移除导师
    pub fn remove_mentor(&mut self) {
        // TODO: 发送移除导师请求
        // MirNetwork.Network.Enqueue(new ClientPackets.RemoveMentor());
        println!("Sending remove mentor request");
    }

    /// 处理每帧更新
    pub fn process(&mut self) {
        if let Some(next_time) = self.next_request_time {
            if Instant::now() >= next_time {
                self.next_request_time = None;
                // TODO: 处理延迟请求
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mentor_dialog_creation() {
        let dialog = MentorDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.mentor_name, "");
        assert_eq!(dialog.mentee_name, "");
        assert!(!dialog.is_mentor);
    }

    #[test]
    fn test_mentor_dialog_show_hide() {
        let mut dialog = MentorDialog::new();
        dialog.show();
        assert!(dialog.visible);
        dialog.hide();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_mentor_dialog_toggle() {
        let mut dialog = MentorDialog::new();
        dialog.toggle();
        assert!(dialog.visible);
        dialog.toggle();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_receive_mentor_info_as_mentor() {
        let mut dialog = MentorDialog::new();
        dialog.receive_mentor_info(
            "MasterWang".to_string(),
            "ApprenticeLi".to_string(),
            50,
            25,
            true,
            false,
            15000,
            true
        );

        assert_eq!(dialog.mentor_name, "MasterWang");
        assert_eq!(dialog.mentee_name, "ApprenticeLi");
        assert_eq!(dialog.mentor_level, 50);
        assert_eq!(dialog.mentee_level, 25);
        assert!(dialog.mentor_online);
        assert!(!dialog.mentee_online);
        assert_eq!(dialog.exp_points, 15000);
        assert!(dialog.is_mentor);
    }

    #[test]
    fn test_receive_mentor_info_as_mentee() {
        let mut dialog = MentorDialog::new();
        dialog.receive_mentor_info(
            "MasterWang".to_string(),
            "ApprenticeLi".to_string(),
            50,
            25,
            true,
            false,
            5000,
            false
        );

        assert_eq!(dialog.mentor_name, "MasterWang");
        assert_eq!(dialog.mentee_name, "ApprenticeLi");
        assert_eq!(dialog.mentor_level, 50);
        assert_eq!(dialog.mentee_level, 25);
        assert!(dialog.mentor_online);
        assert!(!dialog.mentee_online);
        assert_eq!(dialog.exp_points, 5000);
        assert!(!dialog.is_mentor);
    }

    #[test]
    fn test_update_interface_as_mentor() {
        let mut dialog = MentorDialog::new();
        dialog.receive_mentor_info(
            "MasterWang".to_string(),
            "ApprenticeLi".to_string(),
            50,
            25,
            true,
            false,
            15000,
            true
        );

        // Test that update_interface doesn't crash and data is preserved
        dialog.update_interface();
        assert!(dialog.is_mentor);
        assert_eq!(dialog.mentor_name, "MasterWang");
    }

    #[test]
    fn test_update_interface_as_mentee() {
        let mut dialog = MentorDialog::new();
        dialog.receive_mentor_info(
            "MasterWang".to_string(),
            "ApprenticeLi".to_string(),
            50,
            25,
            true,
            false,
            5000,
            false
        );

        dialog.update_interface();
        assert!(!dialog.is_mentor);
        assert_eq!(dialog.mentee_name, "ApprenticeLi");
    }
}