// 绑定点系统（C# CharacterInfo.BindMapIndex/BindLocation）
//
// - SetBind：登录时绑定随机出生安全区（C# PlayerObject.SetBind）
// - 进入安全区（走/跑/被推）：更新绑定点（C# HumanObject.SetBindSafeZone）
// - 供法师 Teleport（MagicTeleport）、DungeonEscape/TownTeleport 卷轴、回城复活使用

use super::*;

impl WorldActor {
    /// 查找一个随机出生安全区（C# Envir.StartPoints 随机取一个）
    pub(crate) fn random_start_point(&self) -> Option<(i32, i32, i32)> {
        let mut points: Vec<(i32, i32, i32)> = self.map_infos.values()
            .flat_map(|mi| mi.safe_zones.iter()
                .filter(|s| s.start_point)
                .map(move |s| (mi.index, s.x, s.y)))
            .collect();
        if points.is_empty() {
            // 无 start_point 配置时回退到任意安全区中心
            points = self.map_infos.values()
                .flat_map(|mi| mi.safe_zones.iter()
                    .map(move |s| (mi.index, s.x, s.y)))
                .collect();
        }
        if points.is_empty() {
            return None;
        }
        let idx = fastrand::usize(0..points.len());
        Some(points[idx])
    }

    /// 登录/进入游戏时确保绑定点有效（C# PlayerObject.SetBind + StartGame 校验）
    pub(crate) async fn ensure_bind(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // 绑定点有效：地图存在且坐标合法（地图未加载则先加载再判定）
        let bind_valid = if let Some(mi) = self.map_infos.get(&state.bind_map_index).cloned() {
            let map_index = mi.index as u16;
            self.get_or_load_map(&mi.file_name, map_index);
            self.maps.get(&map_index)
                .map(|m| m.is_valid(state.bind_x, state.bind_y))
                .unwrap_or(true)
        } else {
            false
        };
        if bind_valid {
            return;
        }
        // 无效 → 随机出生安全区（C# SetBind）
        if let Some((map_index, x, y)) = self.random_start_point() {
            let _ = record.actor_ref.ask(crate::actors::player::SetBind {
                map_index,
                x,
                y,
            }).await;
            debug!("Bind: {} bind set to map {} ({},{})", state.name, map_index, x, y);
        }
    }

    /// 玩家位于安全区内时更新绑定点（C# SetBindSafeZone：绑定点 = 当前地图 + 安全区中心）
    pub(crate) async fn update_bind_safe_zone(&mut self, session_id: u64, map_index: u16, x: i32, y: i32) {
        let center = self.map_infos.get(&(map_index as i32))
            .and_then(|mi| mi.safe_zones.iter()
                .find(|s| {
                    let half = s.size.max(0);
                    (x - s.x).abs() <= half && (y - s.y).abs() <= half
                })
                .map(|s| (s.x, s.y)));
        if let Some((cx, cy)) = center {
            if let Some(record) = self.players.get(&session_id) {
                let _ = record.actor_ref.ask(crate::actors::player::SetBind {
                    map_index: map_index as i32,
                    x: cx,
                    y: cy,
                }).await;
            }
        }
    }

    /// 绑定点地图数据（用于 MagicTeleport / TeleportEscape 的随机范围）
    pub(crate) fn bind_map_size(&mut self, bind_map_index: i32) -> Option<(i32, i32)> {
        let mi = self.map_infos.get(&bind_map_index).cloned()?;
        let map_index = mi.index as u16;
        self.get_or_load_map(&mi.file_name, map_index);
        self.maps.get(&map_index).map(|m| (m.width as i32, m.height as i32))
    }
}
