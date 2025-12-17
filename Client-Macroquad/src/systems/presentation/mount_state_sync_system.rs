use crate::components::{Equipment, MountState, movement::MovementVelocity};
use crate::game::{GameContext, GameResult};
use crate::systems::LogicSystem;
use mir2_shared::enums::ItemType;

#[derive(ecs_macros::LogicSystem)]
pub struct MountStateSyncSystem;

impl MountStateSyncSystem {
    pub fn new() -> Self {
        Self
    }

    // Libraries.rs uses {:02} → Mount/00..99
    const MAX_MOUNT_LIBRARY_INDEX: i16 = 99;

    // 骑乘速度：直接写入 MovementVelocity，保证所有移动路径（含 DirectFollow）一致。
    // 需求：骑马“走路”不需要变快，只让“跑”更快。
    const MOUNT_RUN_SPEED: f32 = 210.0;
    const MOUNT_MAX_SPEED: f32 = 240.0;

    fn derive_mount_index(equipment: &Equipment) -> Option<Option<usize>> {
        let Some(item) = equipment.mount.as_ref() else {
            // 没有“坐骑装备”并不等于“不在骑乘”。
            // 当前客户端的骑乘状态主要由 MountUpdated/ObjectPlayer（协议事件）驱动并写入 MountState。
            // 因此：当装备槽为空时，不在这里覆写 MountState，避免把网络驱动的骑乘状态清空。
            return None;
        };

        let Some(info) = item.info.as_ref() else {
            // mount 存在但 info 尚未到达（网络异步）时，不覆写当前 MountState，避免画面闪烁
            return None;
        };

        if info.item_type != ItemType::Mount {
            return Some(None);
        }

        if !(0..=Self::MAX_MOUNT_LIBRARY_INDEX).contains(&info.shape) {
            return Some(None);
        }

        Some(Some(info.shape as usize))
    }
}

impl LogicSystem for MountStateSyncSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        for (_entity, (equipment, mount_state, velocity)) in ctx.world.query_mut::<(
            &Equipment,
            &mut MountState,
            &mut MovementVelocity,
        )>() {
            let prev = mount_state.mount_index;
            if let Some(derived) = Self::derive_mount_index(equipment) {
                mount_state.mount_index = derived;
            }
            let now = mount_state.mount_index;

            if now.is_some() {
                velocity.walk_speed = crate::components::movement::DEFAULT_WALK_SPEED;
                velocity.run_speed = Self::MOUNT_RUN_SPEED;
                velocity.max_speed = Self::MOUNT_MAX_SPEED;
            } else {
                velocity.walk_speed = crate::components::movement::DEFAULT_WALK_SPEED;
                velocity.run_speed = crate::components::movement::DEFAULT_RUN_SPEED;
                velocity.max_speed = crate::components::movement::DEFAULT_MAX_SPEED;
            }

            if prev != now {
                tracing::info!(
                    "🐎 MountState changed: {:?} -> {:?}; speeds: walk={} run={} max={}",
                    prev,
                    now,
                    velocity.walk_speed,
                    velocity.run_speed,
                    velocity.max_speed
                );
            }
        }
        Ok(())
    }
}
