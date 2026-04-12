/// 天气效果系统
///
/// 职责：
/// - 从 `WeatherState` 组件读取当前天气码
/// - 根据天气码创建/销毁对应的 `ParticleEmitter` 实体
/// - 天气切换时清理旧发射器并创建新的
///
/// 天气码映射（来自 MapChanged.weather / MapInformation.weather_particles）：
/// - 0 = 晴天（无粒子）
/// - 1 = 雨（Rain）
/// - 2 = 雪（Snow）
/// - 3 = 雾（Fog）
/// - 4 = 沙尘（SandStorm）
///
/// 注意：本系统只负责"根据天气码维护粒子发射器"，不直接从网络事件读取天气。
/// 天气码由 `NetworkApplySystem` 从 `MapChanged`/`MapInformation` 包中提取并写入 `WeatherState`。
#[derive(ecs_macros::LogicSystem)]
pub struct WeatherSystem {
    last_weather_code: u16,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSystem {
    pub fn new() -> Self {
        Self { last_weather_code: u16::MAX }
    }

    /// 将天气码映射为粒子类型
    fn weather_to_particle_type(code: u16) -> Option<crate::components::ParticleType> {
        use crate::components::ParticleType;
        match code {
            1 => Some(ParticleType::Rain),
            2 => Some(ParticleType::Snow),
            3 => Some(ParticleType::Fog),
            4 => Some(ParticleType::Sand),
            _ => None,
        }
    }

    /// 销毁旧的天气粒子发射器
    fn destroy_old_emitter(&self, ctx: &mut crate::game::GameContext, old_entity: u64) {
        let _ = ctx.world.despawn(old_entity);
    }

    /// 创建新的天气粒子发射器
    fn create_emitter(
        &self,
        ctx: &mut crate::game::GameContext,
        particle_type: crate::components::ParticleType,
    ) -> Option<u64> {
        use crate::components::{ParticleEmitter, Position};
        use macroquad::prelude::screen_width;

        let emitter = ParticleEmitter {
            emitter_location: Position { x: 0.0, y: -screen_width() * 0.3 },
            generate_particles: true,
            next_particle_time: 0.0,
            spawn_interval: 0.01, // 每 10ms 生成一个粒子
            force_velocity: Position { x: 0.0, y: 0.0 },
            particle_type,
        };
        let entity = ctx.world.spawn((emitter,)).id();
        Some(entity)
    }
}

impl crate::systems::LogicSystem for WeatherSystem {
    fn update(
        &mut self,
        ctx: &mut crate::game::GameContext,
        _dt: f32,
    ) -> crate::game::GameResult {
        use crate::components::WeatherState;

        // 查找 WeatherState 组件
        let Some(weather_state) = ctx.world.iter().find_map(|e| e.get::<&WeatherState>().map(|w| (e.entity(), *w))).map(|(e, w)| (e, w)) else {
            return Ok(());
        };

        let (entity, state) = weather_state;
        let current_code = state.weather_code;

        // 天气未变化，跳过
        if current_code == self.last_weather_code {
            return Ok(());
        }

        // 销毁旧的发射器
        if let Some(old_entity) = state.emitter_entity {
            self.destroy_old_emitter(ctx, old_entity);
        }

        // 根据新天气码创建发射器
        let new_emitter = Self::weather_to_particle_type(current_code)
            .and_then(|pt| self.create_emitter(ctx, pt));

        // 更新 WeatherState
        let new_state = WeatherState {
            weather_code: current_code,
            emitter_entity: new_emitter,
        };
        if let Ok(mut ws) = ctx.world.get::<&mut WeatherState>(entity) {
            ws.weather_code = new_state.weather_code;
            ws.emitter_entity = new_state.emitter_entity;
        }

        if current_code == 0 {
            tracing::debug!("🌤️ Weather cleared (sunny)");
        } else {
            tracing::debug!("🌦️ Weather changed to code {}", current_code);
        }

        self.last_weather_code = current_code;
        Ok(())
    }
}
