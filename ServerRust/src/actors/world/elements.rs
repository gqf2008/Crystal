// 元素系统（弓手 Concentration / ElementalShot / ElementalBarrier）
//
// 对齐 C# `Server/MirObjects/HumanObject.cs`（#region Elemental System）：
// - Concentration：专注 buff（MP 回复），移动/被推时打断 3s；
// - GatherElement：每次命中目标有概率攒元素（需冥想，专注未打断时概率提高）；
// - ObtainElement：更新 ElementsLevel/HasElemental 并广播 SetElemental；
// - 元素等级/上限：C# Settings.OrbsExpList（默认 [50,100,150,200]）。

use super::*;

/// 元素经验档位（C# Settings.OrbsExpList 默认：Orb i = i*50）
pub(crate) const ORBS_EXP_LIST: [i32; 4] = [50, 100, 150, 200];
/// 元素攻击加成（C# Settings.OrbsDmgList 默认：Orb i = i*4）
pub(crate) const ORBS_DMG_LIST: [i32; 4] = [4, 8, 12, 16];
/// 元素防御加成（C# Settings.OrbsDefList 默认：Orb i = i*2）
pub(crate) const ORBS_DEF_LIST: [i32; 4] = [2, 4, 6, 8];
/// 冥想每级上限（C# Settings.GatherOrbsPerLevel 默认 true）
pub(crate) const GATHER_ORBS_PER_LEVEL: bool = true;

/// 元素球数量（C# HumanObject.GetElementalOrbCount：满足 ElementsLevel >= OrbsExpList[i]
/// 的档位数；OrbsExpList 升序 [50,100,150,200]，故为升序计数）
pub(crate) fn elemental_orb_count(elements_level: i32) -> usize {
    ORBS_EXP_LIST.iter().filter(|exp| elements_level >= **exp).count()
}

/// 元素球加成（C# HumanObject.GetElementalOrbPower：无元素返回 0；
/// defensive 用 OrbsDefList，否则 OrbsDmgList，取当前球数档位）
pub(crate) fn elemental_orb_power(elements_level: i32, defensive: bool) -> i32 {
    let count = elemental_orb_count(elements_level);
    if count == 0 {
        return 0;
    }
    let list = if defensive { ORBS_DEF_LIST } else { ORBS_DMG_LIST };
    list[count - 1]
}

impl WorldActor {
    /// 当前世界时间（毫秒，近似 C# Envir.Time）
    fn now_ms(&self) -> i64 {
        self.tick_count as i64 * 100
    }

    /// 广播 SetConcentration（C# HumanObject.UpdateConcentration：自己 + 同图广播）
    pub(crate) async fn broadcast_set_concentration(
        &self, object_id: u32, enabled: bool, interrupted: bool, map_index: u16,
    ) {
        let packet = mir2_shared::packets::server::movement::SetConcentration {
            object_id, enabled, interrupted,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let pkt = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::SetConcentration as i16, &body);
            for (sid, r) in &self.players {
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == map_index {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid, data: pkt.clone(),
                        }).await;
                    }
                }
            }
        }
    }

    /// 广播 SetElemental（C# HumanObject.ObtainElement 末尾 Enqueue + Broadcast）
    pub(crate) async fn broadcast_set_elemental(
        &self, object_id: u32, enabled: bool, value: u32, element: u8, expire_time: i64, map_index: u16,
    ) {
        let packet = mir2_shared::packets::server::movement::SetElemental {
            object_id, enabled, value, element, expire_time,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let pkt = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::SetElemental as i16, &body);
            for (sid, r) in &self.players {
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == map_index {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid, data: pkt.clone(),
                        }).await;
                    }
                }
            }
        }
    }

    /// 广播 ObjectEffect（C# CurrentMap.Broadcast，ElementalBarrierUp/Down 等）
    pub(crate) async fn broadcast_object_effect(
        &self, object_id: u32, effect: mir2_shared::enums::SpellEffect, map_index: u16,
    ) {
        let packet = mir2_shared::packets::server::magic_combat::ObjectEffect {
            object_id, effect, effect_type: 0, delay_time: 0, time: 0,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let pkt = build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ObjectEffect as i16, &body);
            for (sid, r) in &self.players {
                if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                    if os.map_index == map_index {
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: *sid, data: pkt.clone(),
                        }).await;
                    }
                }
            }
        }
    }

    /// 消耗元素（C# ElementalShot/ElementalBarrier 命中后 `ElementsLevel=0; ObtainElement(false)`：
    /// 等级归零再 +1 = 1，HasElemental=false，广播 SetElemental）
    pub(crate) async fn consume_elemental(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let _ = record.actor_ref.ask(crate::actors::player::SetElements {
            level: 0,
            has_elemental: false,
        }).await;
        self.obtain_element(session_id, false).await;
    }

    /// 专注打断：玩家移动/被推时调用（C# HumanObject Walk/Run/Pushed）
    pub(crate) async fn interrupt_concentration(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let active = state.buffs.iter().any(|b| matches!(
            b.buff_type, crate::combat::buff::BuffType::MpRegenBoost { .. }));
        if !active { return; }
        let was_interrupted = state.concentration_interrupted;
        let _ = record.actor_ref.ask(crate::actors::player::SetConcentrationInterrupt {
            interrupted: true,
            interrupt_time_ms: self.now_ms() + 3000,
        }).await;
        // C#：仅首次打断时广播（后续只刷新 InterruptTime）
        if !was_interrupted {
            self.broadcast_set_concentration(
                state.object_id, true, true, state.map_index).await;
        }
    }

    /// 攒元素（C# HumanObject.GatherElement：每次命中目标有概率获得元素，需冥想）
    pub(crate) async fn gather_element(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let meditation_lv = state.magics.iter()
            .find(|m| m.spell == (SPELL_MEDITATION as i32 - 3))
            .map(|m| m.level)
            .unwrap_or(0);
        if meditation_lv == 0 { return; } // C#：未学冥想直接 return

        // rnd >= (8 - meditationLvl - concentrateChance) 时获得元素
        let mut chance = 8 - meditation_lv as i32;
        let concentration_active = state.buffs.iter().any(|b| matches!(
            b.buff_type, crate::combat::buff::BuffType::MpRegenBoost { .. }));
        if concentration_active && !state.concentration_interrupted {
            let conc_lv = state.magics.iter()
                .find(|m| m.spell == (SPELL_CONCENTRATION as i32 - 3))
                .map(|m| m.level)
                .unwrap_or(0);
            chance -= conc_lv as i32 + 1;
        }
        if fastrand::i32(0..10) < chance { return; }
        self.obtain_element(session_id, false).await;
    }

    /// 更新元素（C# HumanObject.ObtainElement）
    ///
    /// - `cast=true`：施法凝聚（ElementalShot/ElementalBarrier 无元素时直接获得第一档）
    /// - `cast=false`：命中攒元素（+1，按冥想等级封顶）
    pub(crate) async fn obtain_element(&mut self, session_id: u64, cast: bool) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let meditation_lv = state.magics.iter()
            .find(|m| m.spell == (SPELL_MEDITATION as i32 - 3))
            .map(|m| m.level)
            .unwrap_or(0);
        if meditation_lv == 0 {
            send_system_message(&self.gate_ref, session_id, "需要先学习冥想");
            return;
        }
        let max_orbs = ORBS_EXP_LIST[ORBS_EXP_LIST.len() - 1];
        let orb_idx = (meditation_lv as usize).min(ORBS_EXP_LIST.len() - 1);

        let mut orb_type = 0u8;
        let mut level: i32;
        let mut has_elemental: bool;
        if cast {
            // C# ObtainElement(true)：直接获得第一档元素
            level = ORBS_EXP_LIST[0];
            orb_type = 1;
            if GATHER_ORBS_PER_LEVEL && meditation_lv == 3 {
                // C# 特殊：冥想 Lv3 时先广播第一档，再升到第二档
                self.broadcast_set_elemental(
                    state.object_id, true, ORBS_EXP_LIST[0] as u32, 1,
                    max_orbs as i64, state.map_index,
                ).await;
                level = ORBS_EXP_LIST[1];
                orb_type = 2;
            }
            has_elemental = true;
        } else {
            // C# ObtainElement(false)：命中攒元素，先清 HasElemental 再 +1
            has_elemental = false;
            level = state.elements_level.saturating_add(1);
            if GATHER_ORBS_PER_LEVEL {
                let cap = ORBS_EXP_LIST[orb_idx];
                if level > cap {
                    has_elemental = true;
                    level = cap;
                }
            }
            if level >= ORBS_EXP_LIST[0] { has_elemental = true; }
            for (i, exp) in ORBS_EXP_LIST.iter().enumerate() {
                if *exp == level {
                    orb_type = (i + 1) as u8;
                    break;
                }
            }
        }

        let _ = record.actor_ref.ask(crate::actors::player::SetElements {
            level,
            has_elemental,
        }).await;
        self.broadcast_set_elemental(
            state.object_id, has_elemental, level as u32, orb_type,
            max_orbs as i64, state.map_index,
        ).await;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orb_count_and_power() {
        // C# GetElementalOrbCount：OrbsExpList=[50,100,150,200]
        assert_eq!(elemental_orb_count(0), 0);
        assert_eq!(elemental_orb_count(49), 0);
        assert_eq!(elemental_orb_count(50), 1);
        assert_eq!(elemental_orb_count(100), 2);
        assert_eq!(elemental_orb_count(150), 3);
        assert_eq!(elemental_orb_count(200), 4);
        assert_eq!(elemental_orb_count(999), 4);
        // C# GetElementalOrbPower：攻击 OrbsDmgList=[4,8,12,16]，防御 OrbsDefList=[2,4,6,8]
        assert_eq!(elemental_orb_power(0, false), 0);
        assert_eq!(elemental_orb_power(50, false), 4);
        assert_eq!(elemental_orb_power(100, false), 8);
        assert_eq!(elemental_orb_power(150, false), 12);
        assert_eq!(elemental_orb_power(200, false), 16);
        assert_eq!(elemental_orb_power(50, true), 2);
        assert_eq!(elemental_orb_power(100, true), 4);
        assert_eq!(elemental_orb_power(200, true), 8);
    }
}
