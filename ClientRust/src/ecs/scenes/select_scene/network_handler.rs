//! 网络事件处理模块
//! 负责处理来自服务器的游戏事件响应

use crate::network::handlers::GameEvent;
use crate::ecs::scenes::SceneType;
use super::SelectScene;

impl SelectScene {
    /// 处理网络事件（在 GameApp 中调用）
    /// 
    /// 根据事件类型分发到具体的处理方法
    pub fn handle_network_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::LoginSuccess { characters } => {
                self.handle_login_success(characters);
            }
            GameEvent::Disconnected { reason } => {
                self.handle_disconnected(reason);
            }
            GameEvent::CharacterDeleted { index } => {
                self.handle_delete_character_success(*index as i32);
            }
            GameEvent::CharacterCreated { name } => {
                self.handle_new_character_success_simple(name);
            }
            GameEvent::StartGame { delay } => {
                self.handle_start_game_delay(*delay as i64);
            }
            _ => {
                // 忽略其他事件
            }
        }
    }

    // ========== 登录成功处理 ==========

    /// 处理登录成功并接收角色列表
    fn handle_login_success(&mut self, characters: &[mir2_shared::SelectInfo]) {
        tracing::info!("🎮 收到角色列表: {} 个角色", characters.len());
        
        // 清空现有角色列表
        self.character_select_ui.clear_characters();
        
        // 添加所有角色
        for character in characters {
            tracing::info!("  - 角色: {} (Lv.{} {:?} {:?})", 
                character.name, 
                character.level,
                character.class,
                character.gender
            );
            self.character_select_ui.add_character(character.clone());
        }
        
        tracing::info!("✅ 角色列表加载完成");
    }

    // ========== 系统消息处理 ==========

    fn handle_disconnected(&self, reason: &str) {
        println!("Disconnected: {}", reason);
        // TODO: Return to login scene
    }

    // ========== 删除角色处理 ==========

    /// 处理角色删除成功事件
    fn handle_delete_character_success(&mut self, character_index: i32) {
        tracing::info!("✅ 角色删除成功: index={}", character_index);
        
        // 1. 关闭删除对话框
        self.delete_character_dialog = None;
        
        // 2. 从角色列表移除已删除的角色
        if self.character_select_ui.remove_character_by_index(character_index) {
            tracing::info!("📋 已从列表移除角色 (index={}), 剩余角色数: {}", 
                character_index, self.character_select_ui.character_count());
        }
        
        // TODO: 显示成功消息框 "Your character was deleted successfully."
    }

    // ========== 创建角色处理 ==========

    /// 处理角色创建成功事件 (简化版 - 只有名称)
    fn handle_new_character_success_simple(&mut self, name: &str) {
        tracing::info!("✅ 角色创建成功: {}", name);
        
        // 1. 关闭新建角色对话框
        self.new_character_dialog = None;
        
        // 2. 显示成功消息
        let mut message_box = super::MessageBox::new(
            format!("Character '{}' created successfully!", name),
            super::MessageBoxButtons::Ok,
            super::DESIGN_WIDTH,
            super::DESIGN_HEIGHT
        );
        message_box.show();
        self.message_box = Some(message_box);
        
        // 3. 选中新创建的角色
        self.character_select_ui.select_character(0);
        
        tracing::info!("📋 新角色已添加到列表, 总角色数: {}", self.character_select_ui.character_count());
        
        // TODO: 显示成功消息框 "Your character was created successfully."
    }

    // ========== 开始游戏处理 ==========

    /// 处理玩家生成事件
    /// 
    /// 注意: 某些服务器实现不发送 StartGameResponse，而是直接发送 PlayerSpawned
    fn handle_player_spawned(&mut self, _player: &()) {
        // TODO: PlayerState已从game_client移除，需要使用新的网络事件结构
        // tracing::info!("🎮 玩家已生成: {} (Lv.{}, HP:{}/{}, MP:{}/{})", 
        //     player.name, player.level, player.health, player.max_health, player.mana, player.max_mana);
        // tracing::info!("📍 位置: ({}, {})", 
        //     player.location.x, player.location.y);
        tracing::info!("✅ 切换到游戏场景...");
        
        self.pending_scene_change = Some(SceneType::Game);
    }

    /// 处理开始游戏响应
    /// 
    /// Result codes from Server\MirObjects\PlayerObject.cs:
    /// - 0: AllowStartGame disabled but connection allowed (special case)
    /// - 1: Not logged in
    /// - 2: Character not found
    /// - 3: Failed to start game (validation error)
    /// - 4: Success! (normal case - see StartGameSuccess())
    fn handle_start_game_response(&mut self, result: u8) {
        tracing::info!("🎮 进入游戏响应: result={}", result);
        
        if result == 4 || result == 0 {
            // Success - queue scene transition to game
            tracing::info!("✅ 进入游戏成功! (result={}) 切换到游戏场景...", result);
            self.pending_scene_change = Some(SceneType::Game);
        } else {
            // Error
            let error_msg = match result {
                1 => "You are not logged in.",
                2 => "Character not found.",
                3 => "Failed to start game.",
                _ => &format!("Unknown error occurred (result code: {})", result),
            };
            tracing::error!("❌ 进入游戏失败: {}", error_msg);
            // TODO: 显示错误消息框
        }
    }

    /// 处理开始游戏被禁止事件
    fn handle_start_game_banned(&self, reason: &str, expiry_date: &i64) {
        tracing::warn!("🚫 进入游戏被禁止: reason={}, expiry={}", reason, expiry_date);
        // TODO: 显示封禁消息框（需要将 expiry_date 转换为可读日期）
    }

    /// 处理开始游戏延迟事件
    fn handle_start_game_delay(&self, milliseconds: i64) {
        tracing::info!("⏱️ 进入游戏延迟: {}ms", milliseconds);
        // TODO: 显示延迟提示
    }
}
