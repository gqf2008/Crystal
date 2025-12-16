// ============================================================================
// 测试 GameScene - 直接进入游戏主场景（含地图渲染）
// ============================================================================

use macroquad::miniquad::conf::Platform;
use macroquad::prelude::*;

use client_macroquad::scenes::{GameScene, Scene, SceneTransition};
use client_macroquad::ui::text_renderer::init_chinese_font;

use client_macroquad::components::{Health, LibrarySprite, MirAction, Monster, MonsterAnimState, Position};
use client_macroquad::components::network::{NetworkObjectType, NetworkSync};
use client_macroquad::objects::frames::get_monster_frame;
use client_macroquad::resources::LibraryName;

const WINDOW_WIDTH: i32 = 1024;
const WINDOW_HEIGHT: i32 = 768;

struct SpecialFramesValidator {
    initialized: bool,
    direction: u8,
    fox_stage: u8,
    hellbomb_type: u16,
    dragon: Option<hecs::Entity>,
    cave: Option<hecs::Entity>,
    fox: Option<hecs::Entity>,
    hellbomb: Option<hecs::Entity>,
}

impl Default for SpecialFramesValidator {
    fn default() -> Self {
        Self {
            initialized: false,
            direction: 0,
            fox_stage: 0,
            hellbomb_type: 903,
            dragon: None,
            cave: None,
            fox: None,
            hellbomb: None,
        }
    }
}

impl SpecialFramesValidator {
    fn try_init(&mut self, ctx: &mut client_macroquad::game::GameContext) {
        if self.initialized {
            return;
        }

        let dir = mir2_shared::enums::MirDirection::try_from(self.direction)
            .unwrap_or(mir2_shared::enums::MirDirection::Up);

        // 摆在相机中心附近：默认相机在 (0,0)，因此这里能立刻看到。
        let dragon_pos = Position::new(-120.0, -40.0);
        let cave_pos = Position::new(120.0, -40.0);
        let fox_pos = Position::new(-120.0, 100.0);
        let hell_pos = Position::new(120.0, 100.0);

        self.dragon = Some(ctx.world.spawn((
            NetworkSync::new(900_001, NetworkObjectType::Monster),
            dragon_pos,
            LibrarySprite::new(LibraryName::Monsters(902), 0),
            Monster {
                name: "DragonStatue".to_string(),
                monster_type: 902,
                stage: 0,
                ai_state: client_macroquad::components::MonsterAIState::Idle,
                target_id: None,
            },
            MonsterAnimState::new(dir, MirAction::Standing),
            Health::new(100),
        )));

        self.cave = Some(ctx.world.spawn((
            NetworkSync::new(900_002, NetworkObjectType::Monster),
            cave_pos,
            LibrarySprite::new(LibraryName::Monsters(321), 0),
            Monster {
                name: "CaveStatue".to_string(),
                monster_type: 321,
                stage: 0,
                ai_state: client_macroquad::components::MonsterAIState::Idle,
                target_id: None,
            },
            MonsterAnimState::new(dir, MirAction::Standing),
            Health::new(100),
        )));

        self.fox = Some(ctx.world.spawn((
            NetworkSync::new(900_003, NetworkObjectType::Monster),
            fox_pos,
            LibrarySprite::new(LibraryName::Monsters(134), 0),
            Monster {
                name: "GreatFoxSpirit".to_string(),
                monster_type: 134,
                stage: self.fox_stage,
                ai_state: client_macroquad::components::MonsterAIState::Idle,
                target_id: None,
            },
            MonsterAnimState::new(dir, MirAction::Standing),
            Health::new(100),
        )));

        self.hellbomb = Some(ctx.world.spawn((
            NetworkSync::new(900_004, NetworkObjectType::Monster),
            hell_pos,
            LibrarySprite::new(LibraryName::Monsters(self.hellbomb_type as usize), 0),
            Monster {
                name: format!("HellBomb{}", (self.hellbomb_type - 902).clamp(1, 3)),
                monster_type: self.hellbomb_type,
                stage: 0,
                ai_state: client_macroquad::components::MonsterAIState::Idle,
                target_id: None,
            },
            MonsterAnimState::new(dir, MirAction::Standing),
            Health::new(100),
        )));

        self.initialized = true;
        println!(
            "[validator] spawned special monsters. Controls: Left/Right=Direction  Up/Down=Fox Stage  H=HellBomb Variant"
        );
        self.log_frames();
    }

    fn handle_keys(&mut self, ctx: &mut client_macroquad::game::GameContext) {
        if !self.initialized {
            return;
        }

        let mut changed = false;

        if is_key_pressed(KeyCode::Left) {
            self.direction = self.direction.wrapping_add(7) % 8;
            changed = true;
        }
        if is_key_pressed(KeyCode::Right) {
            self.direction = self.direction.wrapping_add(1) % 8;
            changed = true;
        }

        if is_key_pressed(KeyCode::Up) {
            self.fox_stage = (self.fox_stage + 1) % 5;
            changed = true;
        }
        if is_key_pressed(KeyCode::Down) {
            self.fox_stage = (self.fox_stage + 4) % 5;
            changed = true;
        }

        if is_key_pressed(KeyCode::H) {
            self.hellbomb_type = match self.hellbomb_type {
                903 => 904,
                904 => 905,
                _ => 903,
            };
            changed = true;
        }

        if !changed {
            return;
        }

        let dir = mir2_shared::enums::MirDirection::try_from(self.direction)
            .unwrap_or(mir2_shared::enums::MirDirection::Up);

        let now = std::time::Instant::now();

        for e in [self.dragon, self.cave, self.fox, self.hellbomb].into_iter().flatten() {
            if let Ok(mut s) = ctx.world.get::<&mut MonsterAnimState>(e) {
                s.direction = dir;
                s.start_time = now;
            }
        }

        if let Some(e) = self.fox {
            if let Ok(mut m) = ctx.world.get::<&mut Monster>(e) {
                m.stage = self.fox_stage;
            }
        }

        if let Some(e) = self.hellbomb {
            if let Ok(mut m) = ctx.world.get::<&mut Monster>(e) {
                m.monster_type = self.hellbomb_type;
                m.name = format!("HellBomb{}", (self.hellbomb_type - 902).clamp(1, 3));
            }
            if let Ok(mut spr) = ctx.world.get::<&mut LibrarySprite>(e) {
                spr.library = LibraryName::Monsters(self.hellbomb_type as usize);
                spr.index = 0;
                spr.frame = 0;
            }
        }

        self.log_frames();
    }

    fn log_frames(&self) {
        let dir = mir2_shared::enums::MirDirection::try_from(self.direction)
            .unwrap_or(mir2_shared::enums::MirDirection::Up);

        // 这里只打印 Standing 对应的帧表选择；动画推进/帧索引由 AnimationSystem 驱动。
        let dragon = get_monster_frame(902, MirAction::Standing, dir, 0);
        let cave = get_monster_frame(321, MirAction::Standing, dir, 0);
        let fox = get_monster_frame(134, MirAction::Standing, dir, self.fox_stage);
        let hell = get_monster_frame(self.hellbomb_type, MirAction::Standing, dir, 0);

        let unpack = |f: Option<&client_macroquad::objects::frames::Frame>| -> (i32, i32) {
            f.map(|v| (v.start, v.count)).unwrap_or((-1, 0))
        };

        let (dragon_start, dragon_count) = unpack(dragon);
        let (cave_start, cave_count) = unpack(cave);
        let (fox_start, fox_count) = unpack(fox);
        let (hell_start, hell_count) = unpack(hell);

        println!(
            "[validator] dir={} fox_stage={} hellbomb={} | dragon(start={},count={}) cave(start={},count={}) fox(start={},count={}) hell(start={},count={})",
            self.direction,
            self.fox_stage,
            self.hellbomb_type,
            dragon_start,
            dragon_count,
            cave_start,
            cave_count,
            fox_start,
            fox_count,
            hell_start,
            hell_count,
        );
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2 - GameScene 测试".to_string(),
        window_width: WINDOW_WIDTH,
        window_height: WINDOW_HEIGHT,
        window_resizable: true,
        high_dpi: false,
        fullscreen: false,
        platform: Platform {
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🎮 传奇2 - GameScene 测试（专用帧集验证器）");

    // 统一资源根目录：避免从不同工作目录启动时找不到 Data/
    let data_dir = format!("{}/Data", env!("CARGO_MANIFEST_DIR"));
    client_macroquad::resources::resource_manager::set_data_path(&data_dir);
    client_macroquad::resources::libraries::set_data_path(data_dir);

    // 初始化中文字体（MainDialog/各对话框会用到）
    init_chinese_font().await;

    let mut scene = GameScene::new();
    scene.load_textures();
    scene.on_enter().ok();

    let mut validator = SpecialFramesValidator::default();
    validator.try_init(scene.debug_ecs_ctx_mut());

    loop {
        let dt = get_frame_time();

        if let Err(e) = scene.handle_input() {
            eprintln!("❌ 输入处理错误: {}", e);
        }

        // 注入 debug 控制：不加 UI 面板，只用键盘切换并打印日志。
        validator.handle_keys(scene.debug_ecs_ctx_mut());

        let transition = match scene.update(dt) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("❌ 场景更新错误: {}", e);
                SceneTransition::None
            }
        };

        if let Err(e) = scene.render() {
            eprintln!("❌ 场景渲染错误: {}", e);
        }

        if matches!(transition, SceneTransition::Exit) || is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }
}
