// ============================================================================
// 罗盘（C# CompassDialog 源码级对齐）
// 参考：Client/MirScenes/Dialogs/CompassDialog.cs
//   - 容器 (487,264) = (ScreenWidth/2-25, ScreenHeight/2-120)，NotControl 不可点
//   - 40 帧 Prguse2[1470..1509] 方向动画（Process 每帧换 Index）：
//       xDiff = 玩家.x-目标.x、yDiff = 玩家.y-目标.y（瓦片坐标）
//       degree = (atan2(-xDiff, yDiff)*180/PI + 360) % 360
//       index  = 1470 + floor(40/360 * degree)
//   - UseOffSet=true：每帧按各自 Lib 偏移绘制（探针实测 1470: off(9,1)、
//     1480/1490: off(9,10)、1500: off(0,10)），帧尺寸也各不同（20x27~28x18）
//   - 服务端驱动显隐：无目标或已到目标瓦片 → 隐藏（Process :44-48）
// ============================================================================

use bevy::prelude::*;

use crate::map_renderer::GameLibraries;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{spawn_ui_sprite, ui_image, UiFont, UiImageCache};

/// 罗盘容器 X（C# CompassDialog.cs:14：ScreenWidth/2 - 25 = 512 - 25）
pub const COMPASS_X: f32 = 487.0;
/// 罗盘容器 Y（C# CompassDialog.cs:14：ScreenHeight/2 - 120 = 384 - 120）
pub const COMPASS_Y: f32 = 264.0;
/// 箭头动画首帧索引（C# CompassDialog.cs:61：1470 + floor(...)）
pub const COMPASS_BASE: usize = 1470;
/// 箭头动画帧数（C# CompassDialog.cs:59：40/360 * degree → Prguse2[1470..1509]）
pub const COMPASS_FRAMES: usize = 40;

/// #250 罗盘箭头（40 帧 Prguse2[1470..1509]，每帧带 Lib 偏移对齐 C# UseOffSet）
#[derive(Component)]
pub struct CompassArrow {
    /// (图像句柄, Lib 偏移 ox, Lib 偏移 oy)——C# MirImageControl.cs:7
    /// DisplayLocation = Location + GetOffSet(Index)，帧切换时位置同步变
    frames: Vec<(Handle<Image>, f32, f32)>,
}

/// #250 罗盘目标状态（S.SetCompass 写入）
#[derive(Resource, Default)]
pub struct CompassState {
    pub target: Option<(i32, i32)>,
}

/// C# CompassDialog.Process :52-61 帧选择（纯函数）：
/// xDiff/yDiff = 玩家-目标（瓦片坐标，y 向下）；angle = atan2(-xDiff, yDiff)*180/PI
/// degree = (angle+360)%360；index = 1470 + floor(40/360*degree)
pub fn compass_index(px: i32, py: i32, tx: i32, ty: i32) -> usize {
    let x_diff = f64::from(px - tx);
    let y_diff = f64::from(py - ty);
    let angle = (-x_diff).atan2(y_diff) * 180.0 / std::f64::consts::PI;
    let degree = (angle + 360.0) % 360.0;
    COMPASS_BASE + ((40.0 / 360.0 * degree).floor() as usize)
}

pub struct CompassPlugin;

impl Plugin for CompassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CompassState>();
        app.add_systems(OnEnter(AppState::Game), spawn_compass);
        app.add_systems(OnExit(AppState::Game), cleanup_compass);
        app.add_systems(
            Update,
            (compass_target_system, compass_frame_system)
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

fn cleanup_compass(mut commands: Commands, arrows: Query<Entity, With<CompassArrow>>) {
    for e in arrows.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_compass(
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    _fonts: ResMut<Assets<Font>>,
    _ui_font: ResMut<UiFont>,
) {
    if !crate::ui::sprite_ui::ui_enabled("compass") {
        return;
    }
    libs.0.ensure_initialized();
    // 40 帧一次性装载（C# Process 每帧换 Index；各帧尺寸/偏移不同，随帧记录偏移）
    let mut frames = Vec::with_capacity(COMPASS_FRAMES);
    for i in COMPASS_BASE..COMPASS_BASE + COMPASS_FRAMES {
        let Some(info) = libs.0.get_image(LibraryName::Prguse2, i) else {
            continue;
        };
        let Some(h) = ui_image(&mut libs, &mut images, &mut cache, LibraryName::Prguse2, i) else {
            continue;
        };
        frames.push((h, f32::from(info.offset_x), f32::from(info.offset_y)));
    }
    if let Some((h, ox, oy)) = frames.first() {
        // 首帧 N=1470；位置按帧偏移（C# UseOffSet）；C# Visible=false 默认隐藏
        let e = spawn_ui_sprite(
            &mut commands,
            h.clone(),
            COMPASS_X + ox,
            COMPASS_Y + oy,
            5.0,
            1.0,
        );
        commands
            .entity(e)
            .insert((CompassArrow { frames }, Visibility::Hidden));
    }
}

/// #250：S.SetCompass → 更新目标
fn compass_target_system(
    mut state: ResMut<CompassState>,
    mut events: MessageReader<crate::network::server_event::ServerEvent>,
) {
    for ev in events.read() {
        if let crate::network::server_event::ServerEvent::CompassTarget { x, y } = ev {
            state.target = Some((*x, *y));
            tracing::info!("🧭 [COMPASS] 目标 ({},{})", x, y);
        }
    }
}

/// C# CompassDialog.Process：无目标/无玩家/已到目标瓦片 → 隐藏；否则显形并按方向换帧
fn compass_frame_system(
    state: Res<CompassState>,
    mut arrows: Query<(&mut Sprite, &mut Transform, &mut Visibility, &CompassArrow)>,
    players: Query<
        &Transform,
        (
            With<crate::actor::LocalPlayer>,
            With<crate::actor::NetObjectId>,
            Without<CompassArrow>,
        ),
    >,
) {
    let Ok((mut sprite, mut tf, mut vis, arrow)) = arrows.single_mut() else {
        return;
    };
    let Some((tx, ty)) = state.target else {
        *vis = Visibility::Hidden;
        return;
    };
    // C# :44 目标 Point.Empty((0,0)) 视为无目标
    if (tx, ty) == (0, 0) {
        *vis = Visibility::Hidden;
        return;
    }
    let Ok(pf) = players.single() else {
        *vis = Visibility::Hidden;
        return;
    };
    let (px, py) = crate::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
    // C# :44 玩家已在目标瓦片 → 隐藏
    if (px, py) == (tx, ty) {
        *vis = Visibility::Hidden;
        return;
    }
    let i = compass_index(px, py, tx, ty) - COMPASS_BASE;
    if let Some((h, ox, oy)) = arrow.frames.get(i) {
        if sprite.image != *h {
            // 帧尺寸/偏移各不同 → 换帧同时按帧偏移重定位（C# UseOffSet）
            sprite.image = h.clone();
            tf.translation = Vec3::new(COMPASS_X + ox, -(COMPASS_Y + oy), tf.translation.z);
        }
    }
    *vis = Visibility::Visible;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C# CompassDialog.Process 帧公式字面值（玩家 (100,100)，8 方位目标）
    #[test]
    fn compass_index_matches_csharp_formula() {
        let cases: [((i32, i32), usize); 8] = [
            ((100, 90), 1470),  // N
            ((110, 90), 1475),  // NE
            ((110, 100), 1480), // E
            ((110, 110), 1485), // SE
            ((100, 110), 1490), // S
            ((90, 110), 1495),  // SW
            ((90, 100), 1500),  // W
            ((90, 90), 1505),   // NW
        ];
        for ((tx, ty), want) in cases {
            assert_eq!(compass_index(100, 100, tx, ty), want, "目标 ({tx},{ty})");
        }
    }

    /// 真实生成 + 跑系统：无目标隐藏 → 有目标显形换帧+按帧偏移落位 → 已到隐藏
    #[test]
    fn compass_hide_show_frame_and_offset() {
        use crate::resources::libraries::{resolve_data_path, Libraries};
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.insert_resource(GameLibraries(Libraries::new(resolve_data_path())));
        world.insert_resource(Assets::<Image>::default());
        world.insert_resource(UiImageCache::default());
        world.insert_resource(Assets::<Font>::default());
        world.insert_resource(UiFont::default());
        world.insert_resource(CompassState::default());
        // 玩家站瓦片 (100,100)：tile_to_world 脚点 → world_to_tile 反查 == (100,100)
        let wp = crate::game::movement::tile_to_world(100, 100);
        assert_eq!(crate::game::movement::world_to_tile(wp.x, wp.y), (100, 100));
        world.spawn((
            crate::actor::LocalPlayer,
            crate::actor::NetObjectId(1),
            Transform::from_xyz(wp.x, wp.y, 0.0),
        ));
        world
            .run_system_once(spawn_compass)
            .expect("spawn_compass 应成功");
        let arrow = {
            let mut q = world.query_filtered::<Entity, With<CompassArrow>>();
            q.iter(&world).next().expect("应生成罗盘箭头")
        };

        // 无目标 → 隐藏（C# :44 Destination==Point.Empty）
        world
            .run_system_once(compass_frame_system)
            .expect("compass_frame_system 应成功");
        {
            let mut q = world.query_filtered::<&Visibility, With<CompassArrow>>();
            assert_eq!(*q.get(&world, arrow).unwrap(), Visibility::Hidden);
        }
        // 目标南 (100,110) → 显形 + 帧 1490 + 位置按帧偏移 (9,10)（探针实测）
        world.resource_mut::<CompassState>().target = Some((100, 110));
        world
            .run_system_once(compass_frame_system)
            .expect("compass_frame_system 应成功");
        {
            let mut fq = world.query_filtered::<&CompassArrow, ()>();
            let want = fq.get(&world, arrow).unwrap().frames[1490 - COMPASS_BASE].clone();
            assert_eq!(want.1, 9.0, "S 帧 Lib 偏移 ox（探针实测）");
            assert_eq!(want.2, 10.0, "S 帧 Lib 偏移 oy（探针实测）");
            let mut q =
                world.query_filtered::<(&Sprite, &Transform, &Visibility), With<CompassArrow>>();
            let (sprite, tf, vis) = q.get(&world, arrow).unwrap();
            assert_eq!(*vis, Visibility::Visible);
            assert_eq!(sprite.image, want.0, "帧 = 1490");
            // 绝对落位字面值：487+9=496、264+10=274（防常量漂移，与 ui_alignment 呼应）
            assert_eq!(tf.translation.x, 496.0);
            assert_eq!(tf.translation.y, -274.0);
        }
        // 已到目标瓦片 → 隐藏（C# :44）
        world.resource_mut::<CompassState>().target = Some((100, 100));
        world
            .run_system_once(compass_frame_system)
            .expect("compass_frame_system 应成功");
        {
            let mut q = world.query_filtered::<&Visibility, With<CompassArrow>>();
            assert_eq!(*q.get(&world, arrow).unwrap(), Visibility::Hidden);
        }
        // 目标 (0,0) == Point.Empty → 隐藏
        world.resource_mut::<CompassState>().target = Some((0, 0));
        world
            .run_system_once(compass_frame_system)
            .expect("compass_frame_system 应成功");
        {
            let mut q = world.query_filtered::<&Visibility, With<CompassArrow>>();
            assert_eq!(*q.get(&world, arrow).unwrap(), Visibility::Hidden);
        }
    }
}
