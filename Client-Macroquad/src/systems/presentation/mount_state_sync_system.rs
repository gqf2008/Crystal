use crate::components::{Equipment, MountState};
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

    fn derive_mount_index(equipment: &Equipment) -> Option<Option<usize>> {
        let Some(item) = equipment.mount.as_ref() else {
            return Some(None);
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
        for (_entity, (equipment, mount_state)) in ctx
            .world
            .query_mut::<(&Equipment, &mut MountState)>()
            .into_iter()
        {
            if let Some(derived) = Self::derive_mount_index(equipment) {
                mount_state.mount_index = derived;
            }
        }
        Ok(())
    }
}
