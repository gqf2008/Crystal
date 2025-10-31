//! LoginScene 网络事件处理模块
//! 
//! 负责处理所有与服务器通信相关的事件响应

use crate::network::{GameEvent, NetContext};
use super::{LoginScene, InputField};
use super::message_box::MessageBox;
use std::sync::Arc;

impl LoginScene {
    /// 处理网络事件
    pub fn handle_network_event(&mut self, event: &GameEvent, net_ctx: &Arc<NetContext>, world: &mut hecs::World) {
        match event {
            GameEvent::Connected => {
                self.on_connected(net_ctx);
            }
            GameEvent::ClientVersionResponse { result } => {
                self.on_client_version_response(*result, net_ctx);
            }
            GameEvent::Disconnected { reason } => {
                self.on_disconnected(reason);
            }
            GameEvent::LoginSuccess { characters } => {
                self.on_login_success(characters, world);
            }
            GameEvent::LoginFailed { reason } => {
                self.on_login_failed(reason);
            }
            GameEvent::NewAccountSuccess => {
                self.on_new_account_success();
            }
            GameEvent::NewAccountFailed { reason } => {
                self.on_new_account_failed(reason);
            }
            GameEvent::ChangePasswordSuccess => {
                self.on_change_password_success();
            }
            GameEvent::ChangePasswordFailed { reason } => {
                self.on_change_password_failed(reason);
            }
            _ => {
                // 其他事件由GameApp处理（如LoginSuccess会触发场景切换）
            }
        }
    }
    
    /// 处理连接成功事件
    fn on_connected(&mut self, net_ctx: &Arc<NetContext>) {
        tracing::info!("✅ Connected to server successfully!");
        self.connecting = false;
    }
    
    /// 处理ClientVersion验证响应
    fn on_client_version_response(&mut self, result: u8, net_ctx: &Arc<NetContext>) {
        if result == 1 {
            tracing::info!("✅ ClientVersion验证成功,允许登录");
            self.version_verified = true;
            // 如果之前有用户尝试过登录但由于版本未验证而被缓存,现在自动发送
            if let Some(cmd) = self.pending_login.take() {
                match net_ctx.send(cmd) {
                    Ok(()) => {
                        tracing::info!("📤 已发送缓存的登录请求");
                        self.connecting = true;
                        self.login_enabled = false;
                    }
                    Err(e) => {
                        tracing::error!("❌ 发送缓存的登录请求失败: {}", e);
                        self.show_message("网络错误，无法发送登录请求");
                        self.connecting = false;
                        self.login_enabled = true;
                    }
                }
            }
        } else {
            tracing::error!("❌ ClientVersion验证失败,版本不匹配");
            self.version_verified = false;
            self.show_message("版本错误，请更新客户端");
        }
    }
    
    /// 处理断开连接事件
    fn on_disconnected(&mut self, reason: &str) {
        println!("❌ Disconnected: {}", reason);
        self.connecting = false;
        self.login_enabled = true;  // 🔓 断开连接,重新启用登录
        self.version_verified = false; // 🔓 重置版本验证状态
        tracing::info!("🔓 断开连接,重新启用登录按钮");
         self.show_message("🔓 断开连接,重新启用登录按钮");
    }
    
    /// 处理登录失败
    fn on_login_failed(&mut self, reason: &str) {
        self.show_message(&format!("Login failed: {}", reason));
        self.connecting = false;
        self.login_enabled = true;  // 🔓 登录失败,重新启用登录
        tracing::info!("🔓 登录失败,重新启用登录按钮");
         self.show_message("🔓 登录失败,重新启用登录按钮");
    }
    
    /// 处理新建账号成功
    fn on_new_account_success(&mut self) {
        tracing::info!("✅ 账号创建成功");
        self.show_message("Account created successfully! Please login.");
        self.new_account_dialog = None;
    }
    
    /// 处理新建账号失败
    fn on_new_account_failed(&mut self, reason: &str) {
        tracing::info!("❌ 账号创建失败: {}", reason);
        self.show_message(&format!("Account creation failed: {}", reason));
    }
    
    /// 处理修改密码成功
    fn on_change_password_success(&mut self) {
        tracing::info!("✅ 密码修改成功");
        self.show_message("Password changed successfully!");
        self.change_password_dialog = None;
    }
    
    /// 处理修改密码失败
    fn on_change_password_failed(&mut self, reason: &str) {
        tracing::info!("❌ 密码修改失败: {}", reason);
        self.show_message(&format!("Password change failed: {}", reason));
    }
    
    /// 处理登录成功事件
    /// 对应C#: LoginScene::Login(S.LoginSuccess p)
    fn on_login_success(&mut self, characters: &[mir2_shared::SelectInfo], world: &mut hecs::World) {
        tracing::info!("🎉 登录成功,收到 {} 个角色,启动背景动画", characters.len());
        
        // 🆕 正确的ECS架构: 整个角色列表作为一个实体存储
        let entity = world.spawn((crate::ecs::components::CharacterList::new(characters.to_vec()),));
        tracing::info!("💾 角色列表已存入World: Entity({:?}), {} 个角色", entity, characters.len());
        for (i, character) in characters.iter().enumerate() {
            tracing::info!("  - 角色 #{}: {} (Lv.{})", i + 1, character.name, character.level);
        }
        
        self.login_enabled = false;
        self.animation_paused = false;  // C#原版: _background.Animated = true
        self.background_frame = 0;  // 🆕 从第0帧开始播放动画
        self.should_switch_scene = true; // 🆕 标记需要切换场景(但等动画播放完)
        tracing::info!("🎬 开始播放登录成功动画(0-18帧)");
    }
}