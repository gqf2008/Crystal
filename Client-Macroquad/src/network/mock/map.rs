use super::*;

use crate::resources::map_reader::MapReader;

impl MockNetwork {
    /// 加载地图并发送 MapChanged 事件，并将可走性缓存到 MockWorldState。
    ///
    /// 返回实际采用的出生点（若原出生点不可走，会在附近寻找可走格）。
    pub(super) fn load_and_send_map(
        response_tx: &Sender<NetworkEvent>,
        state: &mut MockWorldState,
        map_path: &str,
        map_index: i32,
        title: &str,
        spawn_x: i32,
        spawn_y: i32,
        direction: u8,
    ) -> (i32, i32) {
        let resolved_path = crate::resources::map_reader::resolve_map_path(map_path);
        tracing::info!("📂 尝试加载地图: {} -> {}", map_path, resolved_path);

        match MapReader::new(&resolved_path) {
            Ok(map_reader) => {
                tracing::info!(
                    "✅ 成功加载地图: {} ({}x{})",
                    resolved_path,
                    map_reader.width,
                    map_reader.height
                );

                // 缓存碰撞：map_cells[x][y] -> map_walkable[y * w + x]
                state.map_width = map_reader.width.max(0);
                state.map_height = map_reader.height.max(0);
                let w = state.map_width as usize;
                let h = state.map_height as usize;
                state.map_walkable.clear();
                state.map_walkable.reserve(w.saturating_mul(h));
                for y in 0..h {
                    for x in 0..w {
                        let walkable = map_reader
                            .map_cells
                            .get(x)
                            .and_then(|col| col.get(y))
                            .map(|c| c.is_walkable())
                            .unwrap_or(true);
                        state.map_walkable.push(if walkable { 1 } else { 0 });
                    }
                }

                // 如果出生点不可走（或地图加载异常导致越界），在附近找一个可走格。
                let mut final_spawn = (spawn_x, spawn_y);
                if !Self::map_is_walkable(state, final_spawn.0, final_spawn.1) {
                    let max_r: i32 = 12;
                    'outer: for r in 1..=max_r {
                        for dy in -r..=r {
                            for dx in -r..=r {
                                // 优先扫描“边框”以更快找到最近点
                                if dx.abs() != r && dy.abs() != r {
                                    continue;
                                }
                                let tx = spawn_x + dx;
                                let ty = spawn_y + dy;
                                if Self::map_is_walkable(state, tx, ty) {
                                    final_spawn = (tx, ty);
                                    break 'outer;
                                }
                            }
                        }
                    }
                }

                // 提取纯文件名（不含路径和扩展名）用于下发 MapChanged
                let file_name = std::path::Path::new(&resolved_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("0")
                    .to_string();

                // 发送 MapChanged 事件 (与 C# Server 格式一致)
                // Mock 随机天气：0=晴天, 1=雨, 2=雪, 3=雾, 4=沙尘
                let weather = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u16).unwrap_or(0)) % 5;
                let _ = response_tx.send(NetworkEvent::MapChanged {
                    packet: mir2_shared::packets::server::MapChanged {
                        map_index,
                        file_name, // 只发送纯文件名 "0"
                        title: title.to_string(),
                        minimap: 0,
                        big_map: 0,
                        lights: 0,
                        location_x: final_spawn.0,
                        location_y: final_spawn.1,
                        direction,
                        map_dark_light: 0,
                        music: 0,
                        weather,
                    },
                });

                final_spawn
            }
            Err(e) => {
                tracing::error!("❌ 加载地图失败 {}: {:?}", map_path, e);
                // 失败时退化：不做碰撞校验
                state.map_width = 0;
                state.map_height = 0;
                state.map_walkable.clear();
                (spawn_x, spawn_y)
            }
        }
    }
}
