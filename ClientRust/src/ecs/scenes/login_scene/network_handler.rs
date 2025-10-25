//! LoginScene 网络事件处理模块
//! 
//! 负责处理所有与服务器通信相关的事件响应

use crate::network::GameEvent;
use super::{LoginScene, InputField};
use super::message_box::MessageBox;

impl LoginScene {
    /// 处理网络事件
    pub fn handle_network_event(&mut self, event: &GameEvent) {
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
                self.on_new_account_response(*result);
            }
            GameEvent::ChangePasswordResponse { result } => {
                self.on_change_password_response(*result);
            }
            GameEvent::LoginSuccess { .. } => {
                self.on_login_success();
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
        self.login_enabled = true;  // 🔓 断开连接,重新启用登录
        tracing::info!("🔓 断开连接,重新启用登录按钮");
        self.message_box = Some(MessageBox::new(
            format!("Disconnected: {}", reason), 
            super::DESIGN_WIDTH, 
            super::DESIGN_HEIGHT
        ));
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
            self.login_enabled = true;  // 🔓 登录失败,重新启用登录
            tracing::info!("🔓 登录失败,重新启用登录按钮");
        }
    }
    
    /// 处理账号封禁通知
    fn on_login_banned(&mut self, reason: &str) {
        self.show_message(&format!("Account banned: {}", reason));
        self.connecting = false;
        self.login_enabled = true;  // 🔓 账号被封禁,重新启用登录
        tracing::info!("🔓 账号被封禁,重新启用登录按钮");
    }
    
    /// 处理新建账号服务器响应
    /// 对应C#: LoginScene::NewAccount(S.NewAccount p)
    fn on_new_account_response(&mut self, result: u8) {
        tracing::info!("📝 收到新建账号响应: result={}", result);
        
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
        
        // 显示消息
        self.show_message(message);
        
        // 根据结果决定是否关闭对话框
        if should_close {
            self.new_account_dialog = None;
        } else if let Some(field) = focus_field {
            // 设置焦点到错误字段
            if let Some(dialog) = &mut self.new_account_dialog {
                dialog.focused_field = field;
                // 如果是AccountID已存在错误,清空字段
                if result == 7 {
                    dialog.registration.account_id.clear();
                }
            }
        }
    }
    
    /// 处理修改密码服务器响应
    /// 对应C#: LoginScene::ChangePassword(S.ChangePassword p)
    fn on_change_password_response(&mut self, result: u8) {
        use super::ChangePasswordResult;
        
        tracing::info!("🔑 收到修改密码响应: result={}", result);
        
        let result_enum = ChangePasswordResult::from_u8(result)
            .unwrap_or(ChangePasswordResult::Disabled);
        
        // 显示服务器返回的消息
        self.show_message(result_enum.message());
        
        // 如果修改成功,关闭对话框
        if result_enum == ChangePasswordResult::Success {
            self.change_password_dialog = None;
            tracing::info!("✅ 密码修改成功!");
        }
        // 错误情况保持对话框打开,让用户修改
    }
    
    /// 处理登录成功事件
    /// 对应C#: LoginScene::Login(S.LoginSuccess p)
    fn on_login_success(&mut self) {
        tracing::info!("🎉 登录成功,启动背景动画");
        self.login_enabled = false;
        self.animation_paused = false;  // C#原版: _background.Animated = true
        // 注意: 场景切换由GameApp处理,这里只负责启动动画
    }
}