// ============================================================================
// AutoPotionSystem - 本地玩家自动喝药（自动回血）
// Priority: priority::AUTO_POTION (116)
// ============================================================================

use std::time::{Duration, Instant};

use mir2_shared::data::item::ItemInfo;
use mir2_shared::enums::ItemType;

use crate::components::{Health, Inventory, LocalPlayer};
use crate::game::{GameContext, GameResult};
use crate::network::handlers::NetworkEvent;
use crate::systems::LogicSystem;

#[derive(ecs_macros::LogicSystem)]
pub struct AutoPotionSystem {
    last_use: Instant,
    cooldown: Duration,
    hp_threshold_ratio: f32,
}

impl Default for AutoPotionSystem {
    fn default() -> Self {
        Self {
            last_use: Instant::now() - Duration::from_secs(10),
            cooldown: Duration::from_millis(800),
            hp_threshold_ratio: 0.40,
        }
    }
}

impl AutoPotionSystem {
    fn is_mp_potion_name(name: &str) -> bool {
        let n = name.to_ascii_lowercase();
        n.contains("mp") || n.contains("mana") || n.contains("魔法") || n.contains("蓝")
    }

    fn hp_potion_score(info: &ItemInfo) -> i32 {
        if info.item_type != ItemType::Potion {
            return i32::MIN / 2;
        }

        // 粗略按名称判断：尽量只喝“红/回血/太阳水”。
        // （服务端最终决定效果；这里仅用于挑选/避免浪费蓝药）
        let name = info.name.as_str();
        if Self::is_mp_potion_name(name) {
            return -1000;
        }

        let mut score = 0;
        if name.contains("金创")
            || name.contains("生命")
            || name.contains("红")
            || name.to_ascii_lowercase().contains("hp")
        {
            score += 100;
        }
        if name.contains("太阳") || name.contains("疗伤") {
            score += 80;
        }
        if name.contains("强效") {
            score += 10;
        }
        if name.contains("大") {
            score += 5;
        }

        // 保底：允许未知名称但为 Potion。
        score
    }

    fn pick_best_hp_potion_unique_id(inv: &Inventory) -> Option<u64> {
        let mut best: Option<(i32, u64)> = None;

        for slot in inv.items.iter() {
            let Some(item) = slot.as_ref() else {
                continue;
            };
            let Some(info) = item.info.as_ref() else {
                continue;
            };
            if info.item_type != ItemType::Potion {
                continue;
            }

            let score = Self::hp_potion_score(info);
            match best {
                None => best = Some((score, item.unique_id)),
                Some((best_score, _)) if score > best_score => best = Some((score, item.unique_id)),
                _ => {}
            }
        }

        best.map(|(_, uid)| uid)
    }
}

impl LogicSystem for AutoPotionSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        let now = Instant::now();
        if now.duration_since(self.last_use) < self.cooldown {
            return Ok(());
        }

        // 找到本地玩家
        let Some(player_entity) = ctx
            .world
            .iter()
            .find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity()))
        else {
            return Ok(());
        };

        let Ok(health) = ctx.world.get::<&Health>(player_entity) else {
            return Ok(());
        };

        if health.current <= 0 || health.max <= 0 {
            return Ok(());
        }

        let hp_ratio = (health.current as f32) / (health.max as f32);
        if hp_ratio >= self.hp_threshold_ratio {
            return Ok(());
        }

        let Ok(inv) = ctx.world.get::<&Inventory>(player_entity) else {
            return Ok(());
        };

        let Some(unique_id) = Self::pick_best_hp_potion_unique_id(&inv) else {
            return Ok(());
        };

        // 发包（server-authoritative）；未连接则直接跳过。
        if let Some(net) = ctx.net() {
            let _ = net.send(NetworkEvent::UseItemRequest { unique_id });
        }

        // 无论是否发包成功，都做冷却，避免断线/Mock未实现时刷屏。
        self.last_use = now;

        Ok(())
    }
}
