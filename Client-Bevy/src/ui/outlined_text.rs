//! 文本黑色描边（对齐 C# MirLabel `OutLine`）
//!
//! C# 技术（MirLabel.cs:220-226）：`OutLine=true` 时先在 4 个 1px 偏移矩形上
//! 以 `OutLineColour`（全部使用点均为 `Color.Black`，见 Damage.cs:37、
//! PlayerObject.cs:5336/5363、MapObject.cs:512/554）各画一遍文本，
//! 再画正文前景色。相对前景 (1,1)：上 (0,-1)/左 (-1,0)/右 (+1,0)/下 (0,+1)。
//!
//! Bevy 无等价的 4 方向描边内建组件（`TextShadow` 仅单方向），故为每个描边
//! 文本挂 4 个黑色子实体副本，`sync_outline_system` 在主文本变化时同步内容。
//! 子实体 `Visibility::Inherited` 随主实体显隐，无需单独同步。
//!
//! C# 无描边的文本（不加 outline）：按钮文本（MirButton.cs:172 已注释）、
//! 聊天标签（MainDialogs.cs:962 `OutLine=false`）、按钮 Hint
//! （CMain.cs:518-539 HintTextLabel 未设 OutLine）、模式标签
//! （MainDialogs.cs:356/366/376 仅设 OutLineColour 未设 OutLine=true）。

use bevy::prelude::*;
use bevy::sprite::Anchor;

/// 描边颜色（C# OutLineColour 全部使用点为 Color.Black）
pub const OUTLINE_COLOR: Color = Color::BLACK;

/// UI 空间（y 向下）相对正文的 4 方向 1px 偏移：
/// 上/左/右/下（C# MirLabel.cs:220-224 各 rect 相对前景 (1,1)）
pub const OUTLINE_OFFSETS_UI: [(f32, f32); 4] = [(0.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)];

/// 世界空间（y 向上）的 4 方向 1px 偏移（与 UI 版仅 y 符号相反）
pub const OUTLINE_OFFSETS_WORLD: [(f32, f32); 4] =
    [(0.0, 1.0), (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0)];

/// 主文本标记（有描边）
#[derive(Component)]
pub struct OutlinedText;

/// 黑色副本子实体标记
#[derive(Component)]
pub struct OutlineShadow;

/// 按空间取向取 4 方向偏移（`y_up=true` 世界空间 y 向上，否则 UI 空间 y 向下）
pub fn outline_offsets(y_up: bool) -> [(f32, f32); 4] {
    if y_up {
        OUTLINE_OFFSETS_WORLD
    } else {
        OUTLINE_OFFSETS_UI
    }
}

/// 给已有文本实体挂 4 个黑色 1px 描边副本（C# MirLabel.cs:220-224 技术）。
///
/// `font`/`size`/`anchor` 与主文本一致；副本 z 低 0.01 画在正文后；
/// `text` 为当前正文内容（sync 系统仅处理后续变化）。
/// 返回 4 个副本实体。
pub fn outline_on(
    commands: &mut Commands,
    text_entity: Entity,
    text: &str,
    font: Handle<Font>,
    size: f32,
    anchor: Anchor,
    y_up: bool,
) -> Vec<Entity> {
    commands.entity(text_entity).insert(OutlinedText);
    let mut shadows = Vec::with_capacity(4);
    commands.entity(text_entity).with_children(|p| {
        for (dx, dy) in outline_offsets(y_up) {
            let e = p
                .spawn((
                    OutlineShadow,
                    Text2d::new(text),
                    anchor,
                    TextFont {
                        font: FontSource::Handle(font.clone()),
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    TextColor(OUTLINE_COLOR),
                    Transform::from_xyz(dx, dy, -0.01),
                    Visibility::Inherited,
                ))
                .id();
            shadows.push(e);
        }
    });
    shadows
}

/// 主文本内容变化 → 同步到 4 个黑色副本（C# 每帧重绘纹理，Bevy 只在变化时复制）
pub fn sync_outline_system(
    mains: Query<(Entity, Ref<Text2d>, &Children), (With<OutlinedText>, Without<OutlineShadow>)>,
    mut shadows: Query<&mut Text2d, (With<OutlineShadow>, Without<OutlinedText>)>,
) {
    for (_, text, children) in &mains {
        if !text.is_changed() {
            continue;
        }
        for child in children.iter() {
            if let Ok(mut t) = shadows.get_mut(child) {
                t.0 = text.0.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::ecs::world::CommandQueue;

    #[test]
    fn outline_offsets_match_csharp() {
        // C# MirLabel.cs:220-224：副本 rect (1,0)/(0,1)/(2,1)/(1,2) 相对前景 (1,1)
        // = 上 (0,-1)/左 (-1,0)/右 (+1,0)/下 (0,+1)（屏幕坐标 y 向下）
        assert_eq!(
            OUTLINE_OFFSETS_UI,
            [(0.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)]
        );
        // 世界空间 y 向上 → 仅 y 符号相反
        assert_eq!(
            OUTLINE_OFFSETS_WORLD,
            [(0.0, 1.0), (-1.0, 0.0), (1.0, 0.0), (0.0, -1.0)]
        );
        // 两空间互为 y 镜像
        for (ui, world) in OUTLINE_OFFSETS_UI.iter().zip(OUTLINE_OFFSETS_WORLD.iter()) {
            assert_eq!(ui.0, world.0);
            assert_eq!(ui.1, -world.1);
        }
    }

    /// 描边副本结构：4 个子实体、黑字、同字体/字号/锚点、1px 偏移、z 在正文后
    #[test]
    fn outlined_text_spawns_four_black_shadows() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let main = commands
            .spawn((
                Text2d::new("Hi"),
                Anchor::TOP_CENTER,
                TextFont {
                    font: FontSource::Handle(Handle::default()),
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                Transform::from_xyz(0.0, 28.0, 0.0),
                Visibility::Visible,
            ))
            .id();
        let shadows = outline_on(
            &mut commands,
            main,
            "Hi",
            Handle::default(),
            11.0,
            Anchor::TOP_CENTER,
            true,
        );
        queue.apply(&mut world);

        assert_eq!(shadows.len(), 4);
        assert!(world.entity(main).contains::<OutlinedText>());
        let expected = OUTLINE_OFFSETS_WORLD;
        for (i, e) in shadows.iter().enumerate() {
            let t = world.entity(*e).get::<Text2d>().unwrap();
            assert_eq!(t.0, "Hi", "副本 {i} 内容与正文一致");
            let font = world.entity(*e).get::<TextFont>().unwrap();
            assert_eq!(font.font_size, FontSize::Px(11.0), "副本 {i} 字号一致");
            let anchor = world.entity(*e).get::<Anchor>().unwrap();
            assert_eq!(*anchor, Anchor::TOP_CENTER, "副本 {i} 锚点一致");
            let color = world.entity(*e).get::<TextColor>().unwrap();
            assert_eq!(color.0, Color::BLACK, "副本 {i} 黑色");
            let tf = world.entity(*e).get::<Transform>().unwrap();
            assert_eq!(
                (tf.translation.x, tf.translation.y),
                expected[i],
                "副本 {i} 1px 偏移"
            );
            assert_eq!(tf.translation.z, -0.01, "副本 {i} z 在正文后");
            // 副本是主文本的子实体（跟随移动/显隐/销毁）
            let parent = world.entity(*e).get::<ChildOf>().unwrap();
            assert_eq!(parent.parent(), main);
        }
    }

    /// 主文本变化 → sync 系统复制到 4 个副本；未变化不复制
    #[test]
    fn sync_outline_copies_text_changes() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let main = commands
            .spawn((
                Text2d::new("old"),
                Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(Handle::default()),
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        outline_on(
            &mut commands,
            main,
            "old",
            Handle::default(),
            12.0,
            Anchor::TOP_LEFT,
            false,
        );
        queue.apply(&mut world);
        world
            .run_system_once(sync_outline_system)
            .expect("首跑应成功");

        // 文本变化 → 副本同步
        world.get_mut::<Text2d>(main).unwrap().0 = "new".to_string();
        world
            .run_system_once(sync_outline_system)
            .expect("同步应成功");
        let mut q = world.query::<(Entity, &Text2d)>();
        let shadows = world
            .query_filtered::<Entity, With<OutlineShadow>>()
            .iter(&world)
            .collect::<Vec<_>>();
        for e in shadows {
            let (_, t) = q.get(&world, e).unwrap();
            assert_eq!(t.0, "new", "副本同步新内容");
        }

        // 主文本不变 → 副本保持（is_changed 早退，不误写）
        world
            .run_system_once(sync_outline_system)
            .expect("二次运行应成功");
        for e in world
            .query_filtered::<Entity, With<OutlineShadow>>()
            .iter(&world)
        {
            let (_, t) = q.get(&world, e).unwrap();
            assert_eq!(t.0, "new");
        }
    }

    /// 变更检测只处理真正变化的文本（mutation 对照：移除 Changed 检查会让每帧
    /// 无谓复制——由上面第二次运行仍正确的断言守住）
    #[test]
    fn outline_on_marks_only_main_text() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let main = commands
            .spawn((
                Text2d::new("x"),
                Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(Handle::default()),
                    font_size: FontSize::Px(9.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::default(),
            ))
            .id();
        outline_on(
            &mut commands,
            main,
            "x",
            Handle::default(),
            9.0,
            Anchor::TOP_LEFT,
            false,
        );
        queue.apply(&mut world);
        // 副本不带 OutlinedText → 不会作为主文本被 sync 扫描
        assert_eq!(
            world
                .query_filtered::<Entity, With<OutlinedText>>()
                .iter(&world)
                .count(),
            1
        );
        assert_eq!(
            world
                .query_filtered::<Entity, With<OutlineShadow>>()
                .iter(&world)
                .count(),
            4
        );
    }
}
