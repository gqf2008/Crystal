//! LoginScene 网络事件处理模块
//! 
//! 负责处理所有与服务器通信相关的事件响应

use crate::network::GameEvent;
use super::{LoginScene, InputField};
use super::message_box::MessageBox;

impl LoginScene {
    /// 处理网络事件
    pub fn handle_network_event(&mut self, event: &GameEvent) {
        tracing::debug!("🎯 LoginScene收到网络事件: {:?}", event);
        
        match event {
            GameEvent::Connected => {
                self.on_connected();
            }
            GameEvent::Disconnected { reason } => {
                self.on_disconnected(reason);
            }
            GameEvent::LoginResponse { result } => {
                self.on_login_response(*result);
            }
            GameEvent::LoginBanned { reason, .. } => {
                self.on_login_banned(reason);
            }
            GameEvent::NewAccountResponse { result } => {
                tracing::info!("🔔 收到NewAccountResponse事件: result={}", result);
                self.on_new_account_response(*result);
            }
            _ => {
                // 其他事件由GameApp处理（如LoginSuccess会触发场景切换）
            }
        }
    }
    
    /// 处理连接成功事件
    fn on_connected(&mut self) {
        println!("✅ Connected to server successfully!");
        self.connecting = false;
    }
    
    /// 处理断开连接事件
    fn on_disconnected(&mut self, reason: &str) {
        println!("❌ Disconnected: {}", reason);
        self.connecting = false;
        self.login_enabled = false;
        self.message_box = Some(MessageBox::new(format!("Disconnected: {}", reason)));
    }
    
    /// 处理登录响应
    fn on_login_response(&mut self, result: u8) {
        if result != 0 {
            let msg = match result {
                1 => "Account not found",
                2 => "Invalid password",
                3 => "Account is banned",
                _ => "Login failed"
            };
            self.show_message(msg);
            self.connecting = false;
        }
    }
    
    /// 处理账号封禁通知
    fn on_login_banned(&mut self, reason: &str) {
        self.show_message(&format!("Account banned: {}", reason));
        self.connecting = false;
    }
    
    /// 处理新建账号服务器响应
    /// 对应C#: LoginScene::NewAccount(S.NewAccount p)
    fn on_new_account_response(&mut self, result: u8) {
        tracing::info!("🎯 开始处理新建账号响应: result={}", result);
        
        // 获取响应信息和UI行为
        let (message, should_close, focus_field) = match result {
            0 => ("Account creation is currently disabled.", true, None),
            1 => ("Your AccountID is not acceptable.", false, Some(InputField::AccountId)),
            2 => ("Your Password is not acceptable.", false, Some(InputField::Password)),
            3 => ("Your E-Mail Address is not acceptable.", false, Some(InputField::Email)),
            4 => ("Your User Name is not acceptable.", false, Some(InputField::Username)),
            5 => ("Your Secret Question is not acceptable.", false, Some(InputField::Question)),
            6 => ("Your Secret Answer is not acceptable.", false, Some(InputField::Answer)),
            7 => {
                // AccountID已存在 - 对话框会在设置焦点时自动清空该字段
                ("An Account with this ID already exists.", false, Some(InputField::AccountId))
            }
            8 => {
                // 注册成功
                tracing::info!("✅ 账号创建成功!");
                ("Your account was created successfully.", true, None)
            }
            _ => ("Unknown error occurred.", true, None),
        };
        
        tracing::info!("📝 显示消息: {}", message);
        
        // 显示消息
        self.show_message(message);
        
        // 根据结果决定是否关闭对话框
        if should_close {
            tracing::info!("🚪 关闭新建账号对话框");
            self.new_account_dialog = None;
        } else if let Some(field) = focus_field {
            // 设置焦点到错误字段
            tracing::info!("🎯 设置焦点到字段: {:?}", field);
            if let Some(dialog) = &mut self.new_account_dialog {
                dialog.focused_field = field;
                // 如果是AccountID已存在错误,清空字段
                if result == 7 {
                    dialog.registration.account_id.clear();
                }
            }
        }
        
        tracing::info!("✅ 新建账号响应处理完成");
    }
}
