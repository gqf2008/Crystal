// ============================================================================
// 顶部浮动系统消息（#2563，C# GameScene OutputLines）
// 参考：C# Client/MirScenes/GameScene.cs
//   - :272 10 个 MirLabel；:398-407 构造——Font 10F、LimeGreen、
//     Location (20, 25+i*13)、**OutLine=true**（:406）
//   - :460-465 OutputMessage：入队 {Message, ExpireTime=Time+5000, Type}，
//     超 10 条 RemoveAt(0) 丢最旧
//   - :467-503 ProcessOuput 每帧：过期移除；第 i 行 = 第 i 条消息，
//     颜色 Quest=Gold / Guild=DeepPink / 默认 LimeGreen；无消息的行隐藏
//   - :5621 S.SendOutputMessage 只进浮动行（**不进聊天**——旧 Bevy 路由进
//     聊天系误接，ServerRust send_quest_output_message 亦仅任务进度用）
// ============================================================================

use bevy::prelude::*;

use crate::network::server_event::ServerEvent;
use crate::scenes::AppState;
use crate::ui::outlined_text::{outline_on, OutlineShadow};
use crate::ui::sprite_ui::{shared_cjk_font, spawn_ui_text, UiCjkFont, UiEntity, UiFont};

/// 行数（C# :272 `new MirLabel[10]`）
pub const OUTPUT_LINE_COUNT: usize = 10;
/// 首行 y（C# :405 `Location = new Point(20, 25 + i * 13)`）
pub const OUTPUT_X: f32 = 20.0;
pub const OUTPUT_Y0: f32 = 25.0;
/// 行距（C# :405 `25 + i * 13`）
pub const OUTPUT_DY: f32 = 13.0;
/// 存活秒数（C# :462 `ExpireTime = CMain.Time + 5000`）
pub const OUTPUT_TTL_SECS: f32 = 5.0;
/// 队列上限（C# :463-464 `Count > 10 → RemoveAt(0)`）
pub const OUTPUT_MAX: usize = 10;
/// 字号（C# :403 `new Font(Settings.FontName, 10F)` → px=pt）
pub const OUTPUT_FONT_SIZE: f32 = 10.0;
/// z 序：C# Draw 末尾绘制（一切之上）；高于任务追踪面板(20.x)
pub const OUTPUT_Z: f32 = 30.0;

/// 单条消息（C# OutPutMessage）
#[derive(Debug, Clone)]
pub struct OutputMsg {
    pub message: String,
    pub message_type: u8,
    /// 过期时刻（游戏时钟秒）
    pub expire_at: f32,
}

#[derive(Resource, Default)]
pub struct OutputState {
    pub messages: Vec<OutputMsg>,
}

/// 入队（C# GameScene.OutputMessage :460-465：超上限丢最旧）
pub fn push_output(messages: &mut Vec<OutputMsg>, message: String, message_type: u8, now: f32) {
    messages.push(OutputMsg {
        message,
        message_type,
        expire_at: now + OUTPUT_TTL_SECS,
    });
    if messages.len() > OUTPUT_MAX {
        messages.remove(0);
    }
}

/// 过期移除（C# ProcessOuput :469-473；retain 为其收敛等价）
pub fn purge_expired(messages: &mut Vec<OutputMsg>, now: f32) {
    messages.retain(|m| m.expire_at > now);
}

/// 行颜色（C# :480-491：Quest=Gold / Guild=DeepPink / 默认 LimeGreen；
/// 命名色取 .NET Color 定义：Gold(255,215,0)、DeepPink(255,20,147)、LimeGreen(50,205,50)）
pub fn output_line_color(message_type: u8) -> Color {
    use mir2_shared::enums::OutputMessageType;
    if message_type == OutputMessageType::Quest as u8 {
        Color::srgb(1.0, 215.0 / 255.0, 0.0)
    } else if message_type == OutputMessageType::Guild as u8 {
        Color::srgb(1.0, 20.0 / 255.0, 147.0 / 255.0)
    } else {
        Color::srgb(50.0 / 255.0, 205.0 / 255.0, 50.0 / 255.0)
    }
}

/// 第 i 行文本（无消息 = 空串，C# :499）
pub fn output_line_text(messages: &[OutputMsg], i: usize) -> &str {
    messages.get(i).map(|m| m.message.as_str()).unwrap_or("")
}

/// 浮动行标记（index = 行号）
#[derive(Component)]
pub struct OutputLine(pub usize);

/// 全体浮动行（含描边副本子实体，清理用）
#[derive(Component)]
pub struct OutputWidget;

pub struct OutputLinesPlugin;

impl Plugin for OutputLinesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OutputState>();
        app.init_resource::<crate::ui::sprite_ui::UiCjkFont>();
        app.add_systems(OnEnter(AppState::Game), spawn_output_lines);
        app.add_systems(OnExit(AppState::Game), cleanup_output_lines);
        app.add_systems(Update, output_lines_system.run_if(in_state(AppState::Game)));
    }
}

fn cleanup_output_lines(
    mut commands: Commands,
    mut state: ResMut<OutputState>,
    roots: Query<Entity, With<OutputWidget>>,
) {
    // 队列随场景清空（C# 每次 GameScene 新建 OutputMessages 列表——
    // 审查观察：不清空则 5s 内重进 Game 会短暂重现旧消息）
    state.messages.clear();
    for e in roots.iter() {
        commands.entity(e).despawn();
    }
}

/// 10 行 MirLabel（C# :398-407）：黑描边（OutLine=true）+ 默认 LimeGreen（空文本）
fn spawn_output_lines(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut cjk_font: ResMut<UiCjkFont>,
) {
    // #2599 契约：动态中文文本（OutputLines 战报/系统提示）用宋体 CJK 主字体，
    // 而非 Arial——Arial 无 CJK 字形，Han 回退在重排版时退化为 .notdef 豆腐。
    let font = shared_cjk_font(&mut fonts, &mut cjk_font);
    spawn_output_lines_with(&mut commands, &font);
}

/// 真实 spawn（由 spawn_output_lines 调用者传入 Commands；拆出便于测试）
fn spawn_output_lines_with(commands: &mut Commands, font: &Handle<Font>) {
    for i in 0..OUTPUT_LINE_COUNT {
        let y = OUTPUT_Y0 + i as f32 * OUTPUT_DY;
        let e = spawn_ui_text(
            commands,
            font,
            "",
            OUTPUT_X,
            y,
            OUTPUT_FONT_SIZE,
            output_line_color(0),
            OUTPUT_Z,
        );
        commands
            .entity(e)
            .insert((OutputWidget, OutputLine(i), UiEntity));
        // C# :406 OutLine=true：黑描边（UI 空间偏移）
        outline_on(
            commands,
            e,
            "",
            font.clone(),
            OUTPUT_FONT_SIZE,
            bevy::sprite::Anchor::TOP_LEFT,
            false,
        );
    }
}

/// 每帧：收消息入队 → 过期清理 → 10 行文本/颜色/显隐（C# ProcessOuput）
fn output_lines_system(
    time: Res<Time>,
    mut events: MessageReader<ServerEvent>,
    mut state: ResMut<OutputState>,
    // 主行写方（With<OutputLine>；描边副本无该标记，与 shadows 可证互斥）
    mut lines: Query<
        (
            &OutputLine,
            &mut Text2d,
            &mut TextColor,
            &mut Visibility,
            Option<&Children>,
        ),
        With<OutputWidget>,
    >,
    // 描边副本写方（同帧直同步，规避 sync_outline_system 排序依赖）
    mut shadows: Query<&mut Text2d, (With<OutlineShadow>, Without<OutputWidget>)>,
) {
    let now = time.elapsed_secs();
    for ev in events.read() {
        if let ServerEvent::OutputMessage {
            message,
            message_type,
        } = ev
        {
            push_output(&mut state.messages, message.clone(), *message_type, now);
        }
    }
    purge_expired(&mut state.messages, now);

    for (line, mut text, mut color, mut vis, children) in &mut lines {
        match state.messages.get(line.0) {
            Some(m) => {
                if text.0 != m.message {
                    text.0 = m.message.clone();
                    // 正文变化 → 同帧同步描边副本
                    if let Some(children) = children {
                        for child in children.iter() {
                            if let Ok(mut t) = shadows.get_mut(child) {
                                t.0 = m.message.clone();
                            }
                        }
                    }
                }
                let want = output_line_color(m.message_type);
                if color.0 != want {
                    color.0 = want;
                }
                if *vis != Visibility::Visible {
                    *vis = Visibility::Visible;
                }
            }
            None => {
                if !text.0.is_empty() {
                    text.0 = String::new();
                    if let Some(children) = children {
                        for child in children.iter() {
                            if let Ok(mut t) = shadows.get_mut(child) {
                                t.0 = String::new();
                            }
                        }
                    }
                }
                if *vis != Visibility::Hidden {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 常量 == C# 字面值（防漂移）
    #[test]
    fn output_lines_constants_match_csharp() {
        assert_eq!(OUTPUT_LINE_COUNT, 10);
        assert_eq!(OUTPUT_X, 20.0);
        assert_eq!(OUTPUT_Y0, 25.0);
        assert_eq!(OUTPUT_DY, 13.0);
        assert_eq!(OUTPUT_TTL_SECS, 5.0);
        assert_eq!(OUTPUT_MAX, 10);
        assert_eq!(OUTPUT_FONT_SIZE, 10.0);
        // 首行 (20,25)、末行 (20, 25+9*13=142) ⊆ 1024x768 画布
        assert!(OUTPUT_Y0 + 9.0 * OUTPUT_DY < 768.0);
    }

    /// 颜色映射（C# :480-491 命名色 .NET 定义）
    #[test]
    fn output_line_color_matches_csharp() {
        use mir2_shared::enums::OutputMessageType;
        let gold = output_line_color(OutputMessageType::Quest as u8);
        assert!((gold.to_srgba().red - 1.0).abs() < 1e-6);
        assert!((gold.to_srgba().green - 215.0 / 255.0).abs() < 1e-6);
        assert!((gold.to_srgba().blue - 0.0).abs() < 1e-6);

        let pink = output_line_color(OutputMessageType::Guild as u8);
        assert!((pink.to_srgba().red - 1.0).abs() < 1e-6);
        assert!((pink.to_srgba().green - 20.0 / 255.0).abs() < 1e-6);
        assert!((pink.to_srgba().blue - 147.0 / 255.0).abs() < 1e-6);

        let lime = output_line_color(OutputMessageType::Normal as u8);
        assert!((lime.to_srgba().red - 50.0 / 255.0).abs() < 1e-6);
        assert!((lime.to_srgba().green - 205.0 / 255.0).abs() < 1e-6);
        assert!((lime.to_srgba().blue - 50.0 / 255.0).abs() < 1e-6);
    }

    /// 队列：入队带 5s 过期、超 10 丢最旧、过期清理（C# :460-473）
    #[test]
    fn output_queue_semantics() {
        let mut msgs = Vec::new();
        for i in 0..12 {
            push_output(&mut msgs, format!("m{}", i), 3, 100.0);
        }
        // 12 条 → 只剩 10 条且丢的是最旧两条
        assert_eq!(msgs.len(), 10);
        assert_eq!(msgs[0].message, "m2");
        assert_eq!(msgs[9].message, "m11");
        // 过期时刻 = 入队时刻 + 5
        assert!((msgs[0].expire_at - 105.0).abs() < 1e-6);

        // 未到期保留（expire_at > now）；恰好在到期时刻移除（C# Time >= ExpireTime）
        purge_expired(&mut msgs, 104.999);
        assert_eq!(msgs.len(), 10);
        purge_expired(&mut msgs, 105.0);
        assert!(msgs.is_empty(), "同刻入队同刻到期：全部移除");

        // 不同入队时刻 → 分批过期
        let mut msgs = Vec::new();
        push_output(&mut msgs, "early".to_string(), 3, 100.0); // 过期 105
        push_output(&mut msgs, "late".to_string(), 3, 103.0); // 过期 108
        purge_expired(&mut msgs, 106.0);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message, "late");
        purge_expired(&mut msgs, 200.0);
        assert!(msgs.is_empty());

        // 行文本：第 i 条或空（C# :493/:499）
        let mut msgs = Vec::new();
        push_output(&mut msgs, "a".to_string(), 3, 0.0);
        push_output(&mut msgs, "b".to_string(), 3, 0.0);
        assert_eq!(output_line_text(&msgs, 0), "a");
        assert_eq!(output_line_text(&msgs, 1), "b");
        assert_eq!(output_line_text(&msgs, 2), "");
    }

    /// 真实注册冒烟（B0001 + 系统可运行）：两帧不 panic，消息行可见/过期隐藏
    #[test]
    fn output_lines_plugin_updates_without_b0001() {
        let mut app = bevy::app::App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            bevy::state::app::StatesPlugin,
        ));
        app.init_state::<crate::scenes::AppState>();
        app.init_asset::<Font>();
        app.init_resource::<UiFont>();
        // MessageReader<ServerEvent> 需先注册消息（真实 App 由写方系统隐式完成）
        app.add_message::<ServerEvent>();
        app.add_plugins(OutputLinesPlugin);
        // 非 Game 状态跑一帧（schedule 初始化期 B0001 检查）
        app.update();
        // 切 Game 触发 spawn
        app.world_mut()
            .resource_mut::<NextState<crate::scenes::AppState>>()
            .set(crate::scenes::AppState::Game);
        app.update();

        // 10 行 + 每行 4 描边副本
        let mains = app
            .world_mut()
            .query_filtered::<Entity, With<OutputLine>>()
            .iter(app.world())
            .count();
        let shadows = app
            .world_mut()
            .query_filtered::<Entity, With<OutlineShadow>>()
            .iter(app.world())
            .count();
        assert_eq!(mains, OUTPUT_LINE_COUNT);
        assert_eq!(shadows, OUTPUT_LINE_COUNT * 4);

        // 发一条消息 → 第 0 行可见 + 文本/颜色；推 6s → 过期隐藏
        app.world_mut().write_message(ServerEvent::OutputMessage {
            message: "任务进度".to_string(),
            message_type: 4,
        });
        app.update();
        let (text, color, vis) = app
            .world_mut()
            .query_filtered::<(&Text2d, &TextColor, &Visibility), With<OutputLine>>()
            .iter(app.world())
            .find(|(t, _, _)| !t.0.is_empty())
            .map(|(t, c, v)| (t.0.clone(), c.0, *v))
            .expect("应有一行显示");
        assert_eq!(text, "任务进度");
        assert_eq!(color, output_line_color(4));
        assert_eq!(vis, Visibility::Visible);

        // 推虚拟时钟 6s（MinimalPlugins 的 time_system 每帧用 Time<Virtual> 驱动
        // 通用 Time，直接改 Time 会被下一帧覆盖）
        app.world_mut()
            .resource_mut::<bevy::time::Time<bevy::time::Virtual>>()
            .advance_by(std::time::Duration::from_secs_f32(6.0));
        app.update();
        let vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<OutputLine>>()
            .iter(app.world())
            .find(|v| **v == Visibility::Visible);
        assert!(vis.is_none(), "过期后全部隐藏");

        // 退出 Game：实体清空 + 队列清空（C# 每场景新建列表——重进不重现旧消息）
        app.world_mut()
            .resource_mut::<NextState<crate::scenes::AppState>>()
            .set(crate::scenes::AppState::Login);
        app.world_mut()
            .resource_mut::<OutputState>()
            .messages
            .push(OutputMsg {
                message: "残留".to_string(),
                message_type: 3,
                expire_at: f32::MAX,
            });
        app.update();
        let mains = app
            .world_mut()
            .query_filtered::<Entity, With<OutputWidget>>()
            .iter(app.world())
            .count();
        assert_eq!(mains, 0, "退出 Game 实体应清空");
        assert!(
            app.world().resource::<OutputState>().messages.is_empty(),
            "退出 Game 队列应清空"
        );
    }
}
