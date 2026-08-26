//! 文本黑色描边（对齐 C# MirLabel `OutLine`）
//!
//! C# 技术（MirLabel.cs:220-226）：`OutLine=true` 时先在 4 个 1px 偏移矩形上
//! 以 `OutLineColour`（全部使用点均为 `Color.Black`，见 Damage.cs:37、
//! PlayerObject.cs:5335/5362、MapObject.cs:512/554）各画一遍文本，
//! 再画正文前景色。相对前景 (1,1)：上 (0,-1)/左 (-1,0)/右 (+1,0)/下 (0,+1)。
//!
//! 注意 MirLabel 构造器默认 `_outLine = true`（MirLabel.cs:181-182）——C# 中
//! 「未设 OutLine」意味着描边**开启**。显式无描边仅聊天标签
//! （MainDialogs.cs:962/1040 `OutLine=false`）。Bevy 现状：聊天文本无描边
//! ✓ 一致；按钮文本/模式标签/按钮 Hint 在 C# 中有描边而 Bevy 尚未实现
//! → 后续批次补齐。
//!
//! Bevy 无等价的 4 方向描边内建组件（`TextShadow` 仅单方向），故为每个描边
//! 文本挂 4 个黑色副本，sync 系统在主文本变化时同步内容。
//! 世界空间（Text2d）用 [`outline_on`]（副本挂在正文下 + Transform.z=-0.01）；
//! bevy_ui 空间（`Text`/`Node`）用 [`spawn_outlined_label`]——bevy_ui 的
//! `ZIndex` 只决定兄弟次序、子实体恒画在父后，故 UI 版用**兄弟层级**：
//! 副本 `ZIndex(z-1)`、正文 `ZIndex(z)`。
//! 子实体 `Visibility::Inherited` 随主实体显隐，无需单独同步。

use bevy::prelude::*;
use bevy::sprite::Anchor;

/// 描边颜色（C# OutLineColour 全部使用点为 Color.Black）
pub const OUTLINE_COLOR: Color = Color::BLACK;

/// UI 空间 4 个 1px 偏移：直接照搬 C# MirLabel.cs:220-224 各 rect 相对前景
/// (1,1) 的坐标（C# 屏幕系 y 向下）。Bevy UI 实体 y 已翻转，单个条目的视觉
/// 上下方向与 C# 标注相反，但 4 副本同色且方向集合同为 上/左/右/下 → 渲染等价
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

/// 主文本内容变化 → 同步到 4 个黑色副本（C# 每帧重绘纹理，Bevy 只在变化时复制）。
/// 注册位置必须排在所有描边文本写方之后（行会名写方以 `.before(本系统)` 显式
/// 排序，见 actor/mod.rs）：变更检测按 tick 严格比较，同帧晚于本系统的
/// 零散写入将永不可见（描边副本陈旧）
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

/// bevy_ui 文本主标记（有描边，UI 空间版）
#[derive(Component)]
pub struct OutlinedUiText;

/// bevy_ui 黑色描边副本标记
#[derive(Component)]
pub struct OutlineUiShadow;

/// 正文 → 4 个描边副本实体的映射（sync 系统用）
#[derive(Component)]
pub struct OutlineUiShadows(pub Vec<Entity>);

/// 生成带 4 向黑描边的 bevy_ui 文本（C# MirLabel.cs:220-224 技术，UI 空间版）。
///
/// 层级：正文与 4 个副本是**兄弟**（同一父下），副本 `ZIndex(z-1)`、正文
/// `ZIndex(z)`——bevy_ui 的 stacking 中 `ZIndex` 只决定**兄弟间**的相对顺序，
/// **子实体恒画在父之后**（bevy_ui-0.19.1 stack.rs `update_uistack_recursive`：
/// 先 push 父再递归子）。若把副本挂成正文的子实体（本函数最初方案），4 个
/// 位差 ≤1px 的黑副本会整体盖住正文——白/绿字变黑字（批次19-23 F3 修复
/// 教训；世界空间 [`outline_on`] 用 Transform.z=-0.01 无此约束，仍正确）。
///
/// 销毁语义：副本是正文的兄弟而非子实体 → 正文被单独 despawn 时副本不随之
/// 销毁（同根面板 despawn 时一并回收）；sync 对失效 id 取不到即跳过，不崩溃。
pub fn spawn_outlined_label<'a>(
    parent: &'a mut ChildSpawnerCommands,
    font: Handle<Font>,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    color: Color,
    z: i32,
) -> EntityCommands<'a> {
    let mut shadows = Vec::with_capacity(4);
    for (dx, dy) in OUTLINE_OFFSETS_UI {
        let e = parent
            .spawn((
                OutlineUiShadow,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x + dx),
                    top: Val::Px(y + dy),
                    ..default()
                },
                Text::new(text),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(size),
                    ..default()
                },
                TextColor(OUTLINE_COLOR),
                ZIndex(z - 1),
            ))
            .id();
        shadows.push(e);
    }
    parent.spawn((
        OutlinedUiText,
        OutlineUiShadows(shadows),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(x),
            top: Val::Px(y),
            ..default()
        },
        Text::new(text),
        TextFont {
            font: FontSource::Handle(font.clone()),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
        ZIndex(z),
    ))
}

/// bevy_ui 正文内容变化 → 同步到 4 个黑色副本（同 [`sync_outline_system`]）
pub fn sync_outline_ui_system(
    mains: Query<(Ref<Text>, &OutlineUiShadows), (With<OutlinedUiText>, Without<OutlineUiShadow>)>,
    mut shadows: Query<&mut Text, (With<OutlineUiShadow>, Without<OutlinedUiText>)>,
) {
    for (text, list) in &mains {
        if !text.is_changed() {
            continue;
        }
        for id in &list.0 {
            if let Ok(mut t) = shadows.get_mut(*id) {
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
    use bevy::ui::UiStack;

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

    /// bevy_ui 版：4 个黑色副本与正文同父兄弟，副本 ZIndex(z-1)、正文 ZIndex(z)、
    /// 各偏移 ±1px，正文挂 OutlineUiShadows 记录 4 个副本 id
    #[test]
    fn outlined_ui_spawns_four_black_shadows() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut main_id = None;
        commands
            .spawn((
                Node::default(),
                Text::new("root"),
                TextFont {
                    font: FontSource::Handle(Handle::default()),
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ))
            .with_children(|p| {
                main_id = Some(
                    spawn_outlined_label(
                        p,
                        Handle::default(),
                        "Hi",
                        10.0,
                        20.0,
                        11.0,
                        Color::WHITE,
                        1,
                    )
                    .id(),
                );
            });
        queue.apply(&mut world);
        let main = main_id.unwrap();

        let shadows = world
            .entity(main)
            .get::<OutlineUiShadows>()
            .unwrap()
            .0
            .clone();
        assert_eq!(shadows.len(), 4);
        assert!(world.entity(main).contains::<OutlinedUiText>());
        let main_parent = world
            .entity(main)
            .get::<ChildOf>()
            .expect("正文应有父")
            .parent();
        for (i, e) in shadows.iter().enumerate() {
            let t = world.entity(*e).get::<Text>().unwrap();
            assert_eq!(t.0, "Hi", "副本 {i} 内容与正文一致");
            let font = world.entity(*e).get::<TextFont>().unwrap();
            assert_eq!(font.font_size, FontSize::Px(11.0), "副本 {i} 字号一致");
            let color = world.entity(*e).get::<TextColor>().unwrap();
            assert_eq!(color.0, Color::BLACK, "副本 {i} 黑色");
            let node = world.entity(*e).get::<Node>().unwrap();
            assert_eq!(
                (node.left, node.top),
                (
                    Val::Px(10.0 + OUTLINE_OFFSETS_UI[i].0),
                    Val::Px(20.0 + OUTLINE_OFFSETS_UI[i].1)
                ),
                "副本 {i} 1px 偏移"
            );
            let z = world.entity(*e).get::<ZIndex>().unwrap();
            assert_eq!(z.0, 0, "副本 {i} z 比正文（1）低");
            // 兄弟层级：同父（非正文子实体——子实体恒画在父后，会盖住正文）
            let parent = world.entity(*e).get::<ChildOf>().unwrap();
            assert_eq!(parent.parent(), main_parent, "副本 {i} 与正文同父");
        }
        let z = world.entity(main).get::<ZIndex>().unwrap();
        assert_eq!(z.0, 1, "正文 z 高于副本");
    }

    /// bevy_ui 版（P0 回归禁用）：UiStack 顺序——正文严格排在 4 个黑色副本之后
    /// （bevy_ui 0.19.1 子实体恒画在父后、ZIndex 只定兄弟序；若副本误挂为
    /// 正文子实体，黑副本会盖住正文，此处断言会翻转）。
    /// ui_stack_system 未公开导出，故用 App+UiPlugin 跑真实 PostUpdate 栈构建。
    #[test]
    fn outlined_ui_stack_order_shadow_before_main() {
        let mut app = App::new();
        // 最小可跑 UI 栈的插件组合：input（ButtonInput 资源）+ picking（HoverMap 由
        // InteractionPlugin 初始化）+ assets（Image/Font）+ UiPlugin；
        // 全量 DefaultPlugins 太重且依赖窗口事件循环。
        app.add_plugins((
            bevy::asset::AssetPlugin::default(),
            bevy::time::TimePlugin,
            bevy::input::InputPlugin,
            bevy::picking::InteractionPlugin,
            bevy::picking::PickingPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::sprite::SpritePlugin,
            bevy::mesh::MeshPlugin,
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
        ));
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, app.world_mut());
        let mut main_id = None;
        commands.spawn(Node::default()).with_children(|p| {
            main_id = Some(
                spawn_outlined_label(p, Handle::default(), "T", 0.0, 0.0, 11.0, Color::WHITE, 1)
                    .id(),
            );
        });
        queue.apply(app.world_mut());
        app.update();

        let main = main_id.unwrap();
        let shadows = app
            .world()
            .entity(main)
            .get::<OutlineUiShadows>()
            .unwrap()
            .0
            .clone();
        let ui_stack = app.world().resource::<UiStack>();
        let mut main_idx = None;
        let mut shadow_idxs = Vec::new();
        for (i, e) in ui_stack.uinodes.iter().enumerate() {
            if *e == main {
                main_idx = Some(i);
            } else if shadows.contains(e) {
                shadow_idxs.push(i);
            }
        }
        assert_eq!(shadow_idxs.len(), 4, "4 个副本都应在 UiStack 中");
        let main_idx = main_idx.expect("正文应在 UiStack 中");
        let all_shadows_first = shadow_idxs.iter().all(|i| *i < main_idx);
        assert!(
            all_shadows_first,
            "副本应绘制在正文之前：shadow={shadow_idxs:?} main={main_idx}"
        );
    }

    /// bevy_ui 版：正文 Text 变化 → sync 复制到 4 个副本；未变化不复制
    #[test]
    fn sync_outline_ui_copies_text_changes() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut main_id = None;
        commands.spawn(Node::default()).with_children(|p| {
            main_id = Some(
                spawn_outlined_label(p, Handle::default(), "old", 0.0, 0.0, 12.0, Color::WHITE, 1)
                    .id(),
            );
        });
        queue.apply(&mut world);
        let main = main_id.unwrap();
        world
            .run_system_once(sync_outline_ui_system)
            .expect("首跑应成功");

        world.get_mut::<Text>(main).unwrap().0 = "new".to_string();
        world
            .run_system_once(sync_outline_ui_system)
            .expect("同步应成功");
        let shadows = world
            .query_filtered::<Entity, With<OutlineUiShadow>>()
            .iter(&world)
            .collect::<Vec<_>>();
        assert_eq!(shadows.len(), 4);
        let mut q = world.query::<(Entity, &Text)>();
        for e in shadows {
            let (_, t) = q.get(&world, e).unwrap();
            assert_eq!(t.0, "new", "副本同步新内容");
        }

        // 主文本不变 → 副本保持
        world
            .run_system_once(sync_outline_ui_system)
            .expect("二次运行应成功");
        for e in world
            .query_filtered::<Entity, With<OutlineUiShadow>>()
            .iter(&world)
        {
            let (_, t) = q.get(&world, e).unwrap();
            assert_eq!(t.0, "new");
        }
    }
}
