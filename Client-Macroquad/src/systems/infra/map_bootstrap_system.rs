use crate::components::MapData;
use crate::core::GameError;
use crate::game::{GameContext, GameResult};
use crate::resources::MapReader;
use crate::systems::{LogicSystem, MapManager};

/// MapBootstrapSystem
///
/// 目的：在没有任何网络 MapChanged/MapInformation 事件的情况下（例如 `test_game_scene`），
/// 也能自动创建一个初始地图：
/// - 如果不存在 `MapManager`，创建默认的 `MapManager { current_map_file: "n0" }`
/// - 如果不存在 `MapData`，读取地图文件并创建 `MapData`
///
/// 注意：这只是“启动兜底”。正常在线流程仍以 `MapLoadSystem` 为权威。
#[derive(ecs_macros::LogicSystem, Default)]
pub struct MapBootstrapSystem;

impl MapBootstrapSystem {
    const DEFAULT_MAP_FILE: &'static str = "n0";

    fn ensure_map_manager(ctx: &mut GameContext) -> String {
        // 若已存在 MapManager，直接返回当前 file
        {
            let mut q = ctx.world.query::<&MapManager>();
            if let Some((_e, mgr)) = q.iter().next() {
                if !mgr.current_map_file.is_empty() {
                    return mgr.current_map_file.clone();
                }
            }
        }

        // 创建默认 MapManager
        let file = Self::DEFAULT_MAP_FILE.to_string();
        ctx.world.spawn((MapManager {
            current_map_index: -1,
            current_map_file: file.clone(),
            current_map_title: String::new(),
            is_loading: false,
        },));
        file
    }

    fn ensure_map_data(ctx: &mut GameContext, map_file: &str) -> GameResult {
        // 如果已有 MapData，就不重复创建
        if ctx.world.query::<&MapData>().iter().next().is_some() {
            return Ok(());
        }

        if map_file.is_empty() {
            return Ok(());
        }

        let map_path = crate::resources::map_reader::resolve_map_path(map_file);
        match MapReader::new(&map_path) {
            Ok(reader) => {
                ctx.world.spawn((MapData {
                    cells: reader.map_cells.clone(),
                    width: reader.width,
                    height: reader.height,
                },));
                Ok(())
            }
            Err(e) => Err(GameError::ResourceLoadError(format!(
                "MapBootstrapSystem failed to load map {}: {}",
                map_path, e
            ))),
        }
    }
}

impl LogicSystem for MapBootstrapSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        let file = Self::ensure_map_manager(ctx);
        Self::ensure_map_data(ctx, &file)
    }
}
