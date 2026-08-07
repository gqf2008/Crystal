// 配偶/师徒加成（C# PlayerObject.GainExp：Lover/Mentee 同图 + 近距离 + 存活才加经验）
//
// C# Settings.LoverEXPBonus = 5（默认）；Globals.DataRange = 16。
// Mentee 经验加成需 is_mentor 方向标记 + 同组判定，留作后续。

use super::*;

/// 查询配偶经验加成百分比（C# GainExp：HasBuff(Lover) && 配偶同图、InRange(16)、存活）
pub struct GetLoverExpBonus {
    pub session_id: u64,
}

impl Message<GetLoverExpBonus> for WorldActor {
    type Reply = i32;

    async fn handle(&mut self, msg: GetLoverExpBonus, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        // C# Settings.LoverEXPBonus 默认 5
        const LOVER_EXP_BONUS: i32 = 5;
        // C# Globals.DataRange = 16
        const DATA_RANGE: i32 = 16;

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return 0,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return 0,
        };
        let Some(spouse) = state.spouse_name.clone() else {
            return 0;
        };
        for (_, other) in &self.players {
            if let Ok(Some(os)) = other.actor_ref.ask(GetPlayerState).await {
                if os.is_dead || !os.name.eq_ignore_ascii_case(&spouse) {
                    continue;
                }
                if os.map_index != state.map_index {
                    continue;
                }
                let dist = (os.x - state.x).abs() + (os.y - state.y).abs();
                if dist > DATA_RANGE {
                    continue;
                }
                return LOVER_EXP_BONUS;
            }
        }
        0
    }
}
