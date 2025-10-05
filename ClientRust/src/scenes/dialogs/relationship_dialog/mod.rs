//! Relationship Dialog
//!
//! Marriage and relationship management dialog.
//! Corresponds to Client/MirScenes/Dialogs/RelationshipDialog.cs

use std::time::{Duration, Instant};

/// 关系对话框 - 处理婚姻和恋人关系
#[derive(Debug, Clone)]
pub struct RelationshipDialog {
    /// 是否可见
    pub visible: bool,
    /// 窗口位置
    pub location: (i32, i32),

    /// 恋人姓名
    pub lover_name: String,
    /// 结婚日期
    pub date: String,
    /// 地图名称
    pub map_name: String,
    /// 婚姻天数
    pub married_days: i32,
    /// 是否已婚
    pub married: bool,
    /// 恋人是否在线
    pub lover_online: bool,
    /// 下次请求时间
    pub next_request_time: Option<Instant>,
}

impl Default for RelationshipDialog {
    fn default() -> Self {
        Self {
            visible: false,
            location: (400, 300), // Centered on 800x600 screen
            lover_name: String::new(),
            date: String::new(),
            map_name: String::new(),
            married_days: 0,
            married: false,
            lover_online: false,
            next_request_time: None,
        }
    }
}

impl RelationshipDialog {
    /// 创建新的关系对话框
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示对话框
    pub fn show(&mut self) {
        if self.visible {
            return;
        }
        self.visible = true;
        self.request_relationship_info();
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

    /// 请求关系信息
    pub fn request_relationship_info(&mut self) {
        // TODO: 发送网络请求获取关系信息
        // MirNetwork.Network.Enqueue(new ClientPackets.RequestRelationshipInfo());
        self.next_request_time = Some(Instant::now() + Duration::from_secs(1));
    }

    /// 接收关系信息
    pub fn receive_relationship_info(&mut self, lover_name: String, date: String, map_name: String, married_days: i32, married: bool, lover_online: bool) {
        self.lover_name = lover_name;
        self.date = date;
        self.map_name = map_name;
        self.married_days = married_days;
        self.married = married;
        self.lover_online = lover_online;
        self.update_interface();
    }

    /// 更新界面显示
    pub fn update_interface(&mut self) {
        // TODO: Update UI elements based on marriage status
        // This would update labels and button visibility in the actual UI implementation
        if self.married {
            // 已婚状态 - 显示恋人信息，离婚按钮可见
            println!("Married to: {}, Date: {}, Map: {}, Days: {}, Online: {}",
                    self.lover_name, self.date, self.map_name, self.married_days, self.lover_online);
        } else {
            // 未婚状态 - 显示未婚信息，结婚按钮可见
            println!("Not married");
        }
    }

    /// 结婚
    pub fn marry(&mut self) {
        // TODO: 发送结婚请求
        // MirNetwork.Network.Enqueue(new ClientPackets.Marry());
        println!("Sending marry request");
    }

    /// 离婚
    pub fn divorce(&mut self) {
        // TODO: 发送离婚请求
        // MirNetwork.Network.Enqueue(new ClientPackets.Divorce());
        println!("Sending divorce request");
    }

    /// 发送邮件
    pub fn send_mail(&mut self) {
        if self.lover_name.is_empty() {
            return;
        }
        // TODO: 打开邮件对话框发送给恋人
        // GameScene.MailDialog.ComposeMail(self.lover_name);
        println!("Opening mail compose to: {}", self.lover_name);
    }

    /// 发送密语
    pub fn whisper(&mut self) {
        if self.lover_name.is_empty() {
            return;
        }
        // TODO: 发送密语给恋人
        // GameScene.ChatDialog.Whisper(self.lover_name);
        println!("Whispering to: {}", self.lover_name);
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
    fn test_relationship_dialog_creation() {
        let dialog = RelationshipDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.lover_name, "");
        assert!(!dialog.married);
    }

    #[test]
    fn test_relationship_dialog_show_hide() {
        let mut dialog = RelationshipDialog::new();
        dialog.show();
        assert!(dialog.visible);
        dialog.hide();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_relationship_dialog_toggle() {
        let mut dialog = RelationshipDialog::new();
        dialog.toggle();
        assert!(dialog.visible);
        dialog.toggle();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_receive_relationship_info_married() {
        let mut dialog = RelationshipDialog::new();
        dialog.receive_relationship_info(
            "TestLover".to_string(),
            "2023-01-01".to_string(),
            "Town".to_string(),
            365,
            true,
            true
        );

        assert_eq!(dialog.lover_name, "TestLover");
        assert_eq!(dialog.date, "2023-01-01");
        assert_eq!(dialog.map_name, "Town");
        assert_eq!(dialog.married_days, 365);
        assert!(dialog.married);
        assert!(dialog.lover_online);
    }

    #[test]
    fn test_update_interface_married_online() {
        let mut dialog = RelationshipDialog::new();
        dialog.receive_relationship_info(
            "TestLover".to_string(),
            "2023-01-01".to_string(),
            "Town".to_string(),
            365,
            true,
            true
        );

        // Test that update_interface doesn't crash and data is preserved
        dialog.update_interface();
        assert_eq!(dialog.lover_name, "TestLover");
        assert!(dialog.married);
        assert!(dialog.lover_online);
    }

    #[test]
    fn test_update_interface_married_offline() {
        let mut dialog = RelationshipDialog::new();
        dialog.receive_relationship_info(
            "TestLover".to_string(),
            "2023-01-01".to_string(),
            "Town".to_string(),
            365,
            true,
            false
        );

        dialog.update_interface();
        assert_eq!(dialog.lover_name, "TestLover");
        assert!(dialog.married);
        assert!(!dialog.lover_online);
    }

    #[test]
    fn test_update_interface_not_married() {
        let mut dialog = RelationshipDialog::new();
        dialog.receive_relationship_info(
            "".to_string(),
            "".to_string(),
            "".to_string(),
            0,
            false,
            false
        );

        dialog.update_interface();
        assert_eq!(dialog.lover_name, "");
        assert!(!dialog.married);
        assert!(!dialog.lover_online);
    }
}