/// Buff/Debuff 管理系统
///
/// 职责：
/// - 遍历携带 BuffList 组件的实体，更新 Buff 计时
/// - 清理过期的 Buff 并触发 UiCommand 通知 UI
/// - 本地玩家的 Buff 变化同步到 BuffDialog
///
/// Buff 效果由服务器事件驱动（MagicDelayReceived 等），
/// NetworkApplySystem 负责将服务器 Buff 信息写入 ECS 组件。
/// 本系统仅负责客户端侧的计时和过期清理。
use crate::components::{BuffList, LocalPlayer};
use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;
use crate::ui::ui_state::{UiCommand, UiState};

#[derive(ecs_macros::LogicSystem, Default)]
pub struct BufSystem {
    /// 上次更新时间戳（毫秒），用于计算 delta
    last_tick_ms: u64,
}

impl BufSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新 BuffList 中的 Buff 计时，返回已过期的 Buff server ID 列表
    fn update_buff_list(buff_list: &mut BuffList, delta_ms: u64) -> Vec<u32> {
        let mut expired_ids: Vec<u32> = Vec::new();
        buff_list.active_buffs.retain_mut(|buff| {
            let expired = buff.update(delta_ms);
            if expired {
                expired_ids.push(buff.server_buff_id);
            }
            !expired
        });
        expired_ids
    }
}

impl LogicSystem for BufSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let now_ms = (macroquad::time::get_time() * 1000.0) as u64;
        let delta_ms = if self.last_tick_ms > 0 {
            now_ms.saturating_sub(self.last_tick_ms)
        } else {
            0
        };
        self.last_tick_ms = now_ms;

        if delta_ms == 0 {
            return Ok(());
        }

        // 本地玩家的 Buff 计时更新
        let mut expired_buff_ids: Vec<u32> = Vec::new();
        let mut local_player_buffs_changed = false;
        for (buff_list, _local) in ctx.world.query::<(&mut BuffList, &LocalPlayer)>().iter() {
            let before_count = buff_list.active_buffs.len();
            let expired = Self::update_buff_list(buff_list, delta_ms);
            if !expired.is_empty() || buff_list.active_buffs.len() != before_count {
                local_player_buffs_changed = true;
                expired_buff_ids.extend(expired);
            }
        }

        // 非本地玩家/怪物的 BuffList 也更新（不通知 UI）
        for (entity, buff_list) in ctx.world.query::<(hecs::Entity, &mut BuffList)>().iter() {
            if ctx.world.get::<&LocalPlayer>(entity).is_ok() {
                continue;
            }
            let _ = Self::update_buff_list(buff_list, delta_ms);
        }

        // 通知 UI：本地玩家 Buff 变化时同步到 BuffDialog
        if local_player_buffs_changed {
            // 先移除已过期 Buff
            if !expired_buff_ids.is_empty() {
                if let Some(ui) = ctx.world.query::<&UiState>().iter().next() {
                    let mut state = ui.borrow_mut();
                    for buff_id in &expired_buff_ids {
                        state.pending_commands.push(UiCommand::RemoveBuff {
                            buff_type: *buff_id,
                        });
                    }
                }
            }

            // 再同步当前活跃 Buff
            if let Some((_local, buff_list)) =
                ctx.world.query::<(&LocalPlayer, &BuffList)>().iter().next()
            {
                if let Some(ui) = ctx.world.query::<&UiState>().iter().next() {
                    let mut state = ui.borrow_mut();
                    for buff in &buff_list.active_buffs {
                        state.pending_commands.push(UiCommand::AddBuff {
                            buff: crate::scenes::dialogs::game::buff_dialog::BuffEntry {
                                buff_type: buff.server_buff_id,
                                icon_index: 0,
                                name: format!("{:?}", buff.buff_type),
                                remaining_secs: buff.remaining_duration as f32 / 1000.0,
                                is_paused: buff.paused,
                                caster: String::new(),
                            },
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
