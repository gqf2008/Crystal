// player_systems.rs - 玩家实体管理系统
// 
// 功能说明:
// - 玩家属性同步: 将 Player 组件状态同步到 GameSceneState
// - 增益效果管理: 管理 buff 的持续时间、更新和过期移除
//
// 系统列表:
// 1. update_player_stats_system - 玩家属性更新系统
// 2. process_buffs_system - 增益效果处理系统

use bevy::prelude::*;
use super::{Player, GameSceneState};

/// 玩家属性更新系统 - 同步玩家属性到 GameSceneState
/// 
/// 功能:
/// - 监听 Player 组件变化
/// - 更新 GameSceneState 中的玩家等级、生命值
/// - 记录玩家属性变化日志
pub fn update_player_stats_system(
    mut player_query: Query<(&Player, &mut Transform), Changed<Player>>,
    mut game_state: ResMut<GameSceneState>,
) {
    for (player, _transform) in player_query.iter_mut() {
        // 更新游戏状态中的玩家属性
        game_state.player_level = player.level;
        game_state.player_health = 100; // TODO: 从 player.stats 获取
        
        info!(
            "📊 玩家属性已更新: Lv.{} | 攻击力:{} | 防御力:{}",
            player.level, player.stats.attack, player.stats.defense
        );
    }
}

/// 处理增益效果系统 - 管理 buff 的持续时间和过期
/// 
/// 功能:
/// - 更新所有增益的持续时间 (减少 delta_secs)
/// - 移除持续时间 <= 0 的增益
/// - 记录增益消退日志
pub fn process_buffs_system(
    mut player_query: Query<&mut Player>,
    time: Res<Time>,
) {
    for mut player in player_query.iter_mut() {
        if player.buffs.is_empty() {
            continue;
        }
        
        // 更新增益持续时间
        for buff in player.buffs.iter_mut() {
            buff.duration -= time.delta_secs();
        }
        
        // 移除过期增益
        let original_count = player.buffs.len();
        player.buffs.retain(|buff| buff.duration > 0.0);
        
        if player.buffs.len() < original_count {
            info!(
                "💫 增益已消退: {} → {} | 剩余增益: {}",
                original_count,
                player.buffs.len(),
                player.buffs.iter()
                    .map(|b| b.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}
