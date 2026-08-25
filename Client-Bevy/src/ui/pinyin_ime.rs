// ============================================================================
// 游戏内置拼音输入法（自包含 IME）
// ============================================================================
// 背景：系统输入法路线被证伪——bevy/winit 走纯 IMM 桥接，用户的唯一中文输入法
// 手心（纯-TSF TIP, hkl=0）跟 IMM 不配合：候选窗不弹、Preedit 是垃圾。
// 因此游戏自带拼音输入法：自己捕获字母键、查内嵌词典、自绘候选条、选定后
// 把汉字插入聚焦的文本框。完全不依赖系统 IME。
//
// 前提：main.rs 把 Window.ime_enabled 置 false（winit 用 IACE_CHILDREN 解关联
// IME 上下文），字母键便作为原始 KeyboardInput { Key::Character("a") } 到达。
//
// 与各文本框系统的契约：
//   - pinyin_ime_system 跑在 PreUpdate，读 KeyboardInput 更新 IME 状态
//     （Shift 单按切换中/英；中文模式按非密码聚焦框时消费字母/数字/空格/退格/Esc）。
//   - 各文本框系统跑在 Update：先 ime.consumes_key(key) 判断 IME 是否接管该键，
//     接管则跳过；再 ime.take_commit() 注入已选汉字；并回填 ImeFocus（聚焦框矩形，
//     须每帧重写——PreUpdate 会被 clear_ime_focus 清空，而候选条失焦帧即隐藏，
//     只在聚焦变化时写一次的框会让候选条在聚焦期间闪烁/消失）。
//   - pinyin_mode_chip_system 跑在 PostUpdate：聚焦框右侧常显「中/英」模式 chip。
//     系统 IME 被禁用后 OS 输入法工具条不再出现——chip 是它的游戏内等价物
//     （手心/搜狗用户的「中/英指示 + Shift 切换」通用约定），否则内置 IME 无从发现。
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::ui::libpinyin_ime::LibpinyinEngine;

use crate::ui::sprite_ui::{UiEntity, UiFont};

/// 候选条每页数量
const CANDS_PER_PAGE: usize = 9;

// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
// 拼音引擎（libpinyin，与 mir2x 一致）
// ----------------------------------------------------------------------------
// 自包含词典式输入法（PinyinDict + pinyin_dict.txt）已废弃；现在直接封装 GNU libpinyin
// （mir2x 经 vcpkg 引入的 etorth/libpinyin fork + model20 数据），提供整句拼音预测、
// 不完整拼音、模糊纠正、divided/resplit 切分与动态词频。候选窗口用 PinyinBar 自绘。
// FFI 与引擎见 libpinyin_ime.rs；本文件保留 IME 状态机 / 中英切换 / 候选条三层壳。
// ----------------------------------------------------------------------------

#[derive(Resource)]
pub struct PinyinIme {
    /// 中/英模式（false=英文，沿用原有行为）
    enabled: bool,
    /// 当前拼音输入串（原始字母，e2e GetState 真值）
    composing: String,
    /// 当前候选（来自 libpinyin）
    candidates: Vec<String>,
    /// 候选分页
    page: usize,
    /// 本帧要插入聚焦框的已选文本
    commit_pending: Option<String>,
    /// 本帧 IME 用退格编辑过拼音（供文本框判断是否跳过删除缓冲）
    ate_edit: bool,
    /// libpinyin 引擎
    engine: LibpinyinEngine,
}

impl PinyinIme {
    /// libpinyin 系统/用户数据目录。编译期取自 build.rs 注入的 LIBPINYIN_DATA_DIR/CONF_DIR，
    /// 运行时可用环境变量覆盖（便于把数据目录放到游戏资源目录）。
    fn libpinyin_dirs() -> (String, String) {
        let data = std::env::var("LIBPINYIN_DATA_DIR")
            .unwrap_or_else(|_| env!("LIBPINYIN_DATA_DIR").to_string());
        let conf = std::env::var("LIBPINYIN_CONF_DIR")
            .unwrap_or_else(|_| env!("LIBPINYIN_CONF_DIR").to_string());
        (data, conf)
    }

    pub fn new() -> Self {
        let (data, conf) = Self::libpinyin_dirs();
        let engine = LibpinyinEngine::new(&data, &conf).unwrap_or_else(|| {
            panic!(
                "内置拼音 IME 初始化失败：libpinyin 数据/配置目录无效（data={} conf={}）。\
                 请按 mir2x 方式提供 libpinyin 安装（设置 LIBPINYIN_DIR / LIBPINYIN_DATA_DIR / LIBPINYIN_CONF_DIR）",
                data, conf
            )
        });
        Self {
            enabled: false,
            composing: String::new(),
            candidates: Vec::new(),
            page: 0,
            commit_pending: None,
            ate_edit: false,
            engine,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_composing(&self) -> bool {
        !self.composing.is_empty()
    }

    /// 当前是否有候选（组合中的拼音候选，或提交后的联想候选）供选择
    pub fn has_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }

    /// 当前拼音缓冲（e2e 真值：GetState 报告字母是否到达 IME）
    pub fn composing_text(&self) -> &str {
        &self.composing
    }

    /// 候选条左段显示文本：当前组合拼音（剩余未上屏部分）。
    pub fn display_text(&self) -> String {
        self.engine.input().to_string()
    }

    pub fn has_commit(&self) -> bool {
        self.commit_pending.is_some()
    }

    /// 取走本帧待提交文本（文本框在聚焦时调用）
    pub fn take_commit(&mut self) -> Option<String> {
        self.commit_pending.take()
    }

    fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.composing.clear();
        self.candidates.clear();
        self.page = 0;
        self.engine.clear();
    }

    fn feed_letter(&mut self, c: char) {
        self.composing.push(c);
        self.engine.feed(c);
        self.page = 0;
        self.recompute();
    }

    fn backspace(&mut self) {
        if self.composing.pop().is_some() {
            self.ate_edit = true;
            self.page = 0;
            self.engine.backspace();
            self.recompute();
        }
    }

    /// 选中候选：立即上屏选中词，剩余拼音继续组合（审查 B2 修正：选中即上屏 + 剩余续打）。
    fn select(&mut self, idx: usize) {
        let abs = self.page * CANDS_PER_PAGE + idx;
        if let Some((word, consumed)) = self.engine.choose(abs) {
            // 上屏选中词
            if let Some(existing) = &mut self.commit_pending {
                existing.push_str(&word);
            } else {
                self.commit_pending = Some(word);
            }
            // 剩余拼音继续组合（consumed 为已消费的 ASCII 拼音字节数）
            if consumed < self.composing.len() {
                self.composing = self.composing[consumed..].to_string();
                self.engine.set_input(self.composing.clone());
            } else {
                self.composing.clear();
                self.engine.clear();
            }
            self.page = 0;
            self.recompute();
        }
    }

    /// 空格 / Enter：选首个候选（整句/词）；无候选且组合中则上屏原始拼音（逃生口）
    fn commit_default(&mut self) {
        if !self.candidates.is_empty() {
            self.select(0);
        } else if !self.composing.is_empty() {
            let raw = std::mem::take(&mut self.composing);
            self.engine.clear();
            if let Some(existing) = &mut self.commit_pending {
                existing.push_str(&raw);
            } else {
                self.commit_pending = Some(raw);
            }
        }
    }

    fn cancel(&mut self) {
        self.composing.clear();
        self.candidates.clear();
        self.page = 0;
        self.engine.clear();
    }

    fn page_next(&mut self) {
        let max_page = self.candidates.len().saturating_sub(1) / CANDS_PER_PAGE;
        if self.page < max_page {
            self.page += 1;
        }
    }

    fn page_prev(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    fn recompute(&mut self) {
        self.candidates = self.engine.candidates().to_vec();
    }

    /// 返回当前页候选（供 UI 渲染）
    fn page_candidates(&self) -> &[String] {
        let start = self.page * CANDS_PER_PAGE;
        let end = (start + CANDS_PER_PAGE).min(self.candidates.len());
        if start >= self.candidates.len() {
            &[]
        } else {
            &self.candidates[start..end]
        }
    }

    /// 文本框系统据此判断某键是否被 IME 接管（应跳过自身处理）。
    /// 依赖 PreUpdate 已更新好状态。
    pub fn consumes_key(&self, key: &KeyboardInput) -> bool {
        if !self.enabled || key.state != ButtonState::Pressed {
            return false;
        }
        let engaged =
            self.is_composing() || self.has_candidates() || self.commit_pending.is_some() || self.ate_edit;
        match &key.logical_key {
            Key::Character(c) => {
                let s = c.as_str();
                if s.chars().all(|ch| ch.is_ascii_alphabetic()) {
                    // 字母：IME 正在组合时接管（PreUpdate 已把它喂进拼音）
                    self.is_composing()
                } else if s.chars().all(|ch| ch.is_ascii_digit()) {
                    engaged // 数字选候选（组合中）
                } else if (s == "-" || s == "=") && self.has_candidates() {
                    true // 翻页键
                } else {
                    false
                }
            }
            Key::Space => self.is_composing(),
            Key::Backspace => self.is_composing() || self.ate_edit,
            Key::Enter => self.is_composing(),
            Key::Escape => self.is_composing() || self.has_candidates(),
            _ => false,
        }
    }
}

// 聚焦框信息（各文本框系统每帧回填）
// ----------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct ImeFocus {
    /// 聚焦的非密码文本框屏幕矩形 (x,y,w,h)，y 向下；None=无聚焦目标
    pub rect: Option<(f32, f32, f32, f32)>,
}

// ----------------------------------------------------------------------------
// 候选条实体
// ----------------------------------------------------------------------------

#[derive(Component)]
struct PinyinBarBg;

#[derive(Component)]
struct PinyinBarText;

// ----------------------------------------------------------------------------
// 中/英模式 chip 实体（聚焦框右侧）
// ----------------------------------------------------------------------------

#[derive(Component)]
struct PinyinModeChip;

/// chip 与聚焦框右缘的间距（屏幕像素）
const MODE_CHIP_GAP: f32 = 4.0;
/// chip 单字宽 + 余量：聚焦框贴画布右缘时内缩，保证不出 1024 画布
const MODE_CHIP_RIGHT_MARGIN: f32 = 14.0;
/// 画布宽（项目 UI 世界坐标 0..1024×0..768，y 向下）
const MODE_CHIP_CANVAS_W: f32 = 1024.0;

/// chip 文本：中文模式「中」、英文模式「英」（对齐系统输入法指示条的单字约定）
fn mode_chip_text(enabled: bool) -> &'static str {
    if enabled {
        "中"
    } else {
        "英"
    }
}

/// chip 颜色：中文=金（醒目，输入中文的主模式）、英文=灰
fn mode_chip_color(enabled: bool) -> Color {
    if enabled {
        Color::srgb(1.0, 0.85, 0.3)
    } else {
        Color::srgb(0.55, 0.55, 0.55)
    }
}

/// chip 屏幕坐标（左上角，y 向下）：聚焦框右侧、垂直居中；贴右缘时内缩进框内
fn mode_chip_pos(x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
    let cx = (x + w + MODE_CHIP_GAP).min(MODE_CHIP_CANVAS_W - MODE_CHIP_RIGHT_MARGIN);
    let cy = y + h * 0.5 - 6.0;
    (cx, cy)
}

// ----------------------------------------------------------------------------
// 系统
// ----------------------------------------------------------------------------

#[derive(Default)]
struct ShiftToggle {
    down: bool,
    clean: bool,
}

/// PreUpdate：读键盘，更新 IME 状态。跑在各文本框系统（Update）之前。
fn pinyin_ime_system(
    mut keys: MessageReader<KeyboardInput>,
    mut ime: ResMut<PinyinIme>,
    focus: Res<ImeFocus>,
    mut shift: Local<ShiftToggle>,
) {
    // 本帧先清掉上帧的编辑标记（commit_pending 由文本框 take_commit 取走）
    ime.ate_edit = false;

    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();

    // 1) Shift 单按检测（无论是否聚焦都允许切换中/英）
    for key in &key_list {
        if key.logical_key == Key::Shift {
            if key.state == ButtonState::Pressed {
                shift.down = true;
                shift.clean = true;
            } else if key.state == ButtonState::Released {
                if shift.down && shift.clean {
                    ime.toggle();
                    tracing::info!(
                        "[IME] 切换: {}",
                        if ime.enabled() { "中文" } else { "英文" }
                    );
                }
                shift.down = false;
                shift.clean = false;
            }
        } else if key.state == ButtonState::Pressed {
            // 任意非 Shift 按下，则这次 Shift 视作修饰键（不触发切换）
            if shift.down {
                shift.clean = false;
            }
        }
    }

    // 2) 中文模式下，有非密码聚焦框时，消费按键进 IME
    if !ime.enabled() {
        return;
    }
    let focused = focus.rect.is_some();
    if !focused {
        return;
    }
    for key in &key_list {
        if key.state != ButtonState::Pressed {
            continue;
        }
        match &key.logical_key {
            Key::Character(c) => {
                let s = c.as_str();
                if s.chars().all(|ch| ch.is_ascii_alphabetic()) {
                    // 字母进拼音缓冲（小写）
                    for ch in s.chars() {
                        ime.feed_letter(ch.to_ascii_lowercase());
                    }
                } else if s.chars().all(|ch| ch.is_ascii_digit()) {
                    // 数字选候选（组合中或提交后联想候选）
                    if ime.has_candidates() {
                        if let Ok(n) = s.parse::<usize>() {
                            if n >= 1 {
                                ime.select(n - 1);
                            }
                        }
                    }
                } else if s == "-" {
                    if ime.has_candidates() {
                        ime.page_prev();
                    }
                } else if s == "=" {
                    if ime.has_candidates() {
                        ime.page_next();
                    }
                }
            }
            Key::Space => {
                if ime.is_composing() {
                    ime.commit_default();
                } else if ime.has_candidates() {
                    // 联想候选展示中：交还空格给字段（不选候选）
                    ime.cancel();
                }
            }
            Key::Enter => {
                if ime.is_composing() {
                    ime.commit_default();
                } else if ime.has_candidates() {
                    // 联想候选展示中：交还回车给字段（发送），并清掉联想
                    ime.cancel();
                }
            }
            Key::Backspace => {
                if ime.is_composing() {
                    ime.backspace();
                }
            }
            Key::Escape => {
                if ime.is_composing() || ime.has_candidates() {
                    ime.cancel();
                }
            }
            _ => {}
        }
    }
}

/// PreUpdate（在 pinyin_ime_system 之后）：清空 ImeFocus，供 Update 各文本框重新回填。
fn clear_ime_focus(mut focus: ResMut<ImeFocus>) {
    focus.rect = None;
}

/// Update：绘制/更新候选条。全局运行（各 AppState 都可能输入中文）。
/// ParamSet 规避 bg/text 两个查询对 Transform/Visibility 的可写访问冲突。
fn pinyin_candidate_ui_system(
    mut commands: Commands,
    ime: Res<PinyinIme>,
    focus: Res<ImeFocus>,
    ui_font: Res<UiFont>,
    mut bars: ParamSet<(
        Query<(&mut Sprite, &mut Transform, &mut Visibility), With<PinyinBarBg>>,
        Query<(&mut Text2d, &mut Transform, &mut Visibility), With<PinyinBarText>>,
    )>,
) {
    // 确保候选条实体存在（字体就绪后）
    if bars.p0().is_empty() && bars.p1().is_empty() && ui_font.0.is_strong() {
        commands.spawn((
            UiEntity,
            PinyinBarBg,
            Sprite {
                color: Color::srgba(0.0, 0.0, 0.0, 0.8),
                custom_size: Some(Vec2::new(320.0, 18.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 50.0),
            Visibility::Hidden,
        ));
        commands.spawn((
            UiEntity,
            PinyinBarText,
            Text2d::new(""),
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(ui_font.0.clone()),
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(0.4, 1.0, 0.4)),
            Transform::from_xyz(0.0, 0.0, 50.1),
            Visibility::Hidden,
        ));
    }

    // 可见性无条件应用：失焦帧（focus=None）也要把上一帧的 Visible 落回 Hidden，
    // 否则组合中途失焦（如点进密码框）候选条会滞留旧位置（对齐 mode chip 的隐藏路径）。
    // 此处写死 Hidden 而非复用下方 vis：失焦分支的 Hidden 不应依赖可见性条件的合取项
    // （vis 若含 focus 判定，将来「去冗余」删掉该项会把失焦帧写成 Visible，复发本 bug）。
    let Some((x, y, _w, h)) = focus.rect else {
        for (_sprite, _tf, mut v) in bars.p0().iter_mut() {
            *v = Visibility::Hidden;
        }
        for (_text, _tf, mut v) in bars.p1().iter_mut() {
            *v = Visibility::Hidden;
        }
        return;
    };
    // 决定是否显示：中文模式 + 正在组合或有候选（聚焦已由上方 let-else 保证）
    let vis = if ime.enabled() && (ime.is_composing() || ime.has_candidates()) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    // 文本：拼音串 + 候选 1.xxx 2.xxx ...；无组合（联想候选）时仅列候选
    let mut label = if ime.is_composing() {
        ime.display_text() // libpinyin“已选句 + 剩余拼音”，比原始拼音更贴近输入状态
    } else {
        String::new()
    };
    let cands = ime.page_candidates();
    if !cands.is_empty() {
        if !label.is_empty() {
            label.push_str("  ");
        }
        for (i, c) in cands.iter().enumerate() {
            label.push_str(&format!("{}.{} ", i + 1, c));
        }
    }

    let bar_y = y + h + 2.0; // 屏幕坐标 y 向下：输入框下方
    for (mut _sprite, mut tf, mut v) in bars.p0().iter_mut() {
        tf.translation.x = x + 160.0; // 背景以中心定位 → 加半宽
        tf.translation.y = -(bar_y + 9.0); // 加半高后取负转世界坐标
        *v = vis;
    }
    for (mut text, mut tf, mut v) in bars.p1().iter_mut() {
        // 变化才更新，避免每帧重排文本（ICU4X 报错 + CPU，#31）
        if text.0 != label {
            text.0 = label.clone();
        }
        tf.translation.x = x + 4.0;
        tf.translation.y = -(bar_y + 2.0);
        *v = vis;
    }
}

/// PostUpdate：聚焦框右侧的中/英模式 chip。与候选条同帧契约（Update 回填 ImeFocus
/// 之后）。无聚焦框时隐藏；文本/颜色变化才更新（避免每帧重排文本，见候选条 #31 注）。
fn pinyin_mode_chip_system(
    mut commands: Commands,
    ime: Res<PinyinIme>,
    focus: Res<ImeFocus>,
    ui_font: Res<UiFont>,
    mut chips: Query<
        (&mut Text2d, &mut TextColor, &mut Transform, &mut Visibility),
        With<PinyinModeChip>,
    >,
) {
    // 确保 chip 实体存在（字体就绪后；UiFont 补强由候选条系统负责）
    if chips.is_empty() && ui_font.0.is_strong() {
        commands.spawn((
            UiEntity,
            PinyinModeChip,
            Text2d::new(""),
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(ui_font.0.clone()),
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(mode_chip_color(ime.enabled())),
            Transform::from_xyz(0.0, 0.0, 50.2),
            Visibility::Hidden,
        ));
    }

    let Some((x, y, w, h)) = focus.rect else {
        for (_t, _c, _tf, mut v) in &mut chips {
            *v = Visibility::Hidden;
        }
        return;
    };
    let (cx, cy) = mode_chip_pos(x, y, w, h);
    let want = mode_chip_text(ime.enabled());
    for (mut text, mut color, mut tf, mut v) in &mut chips {
        if text.0 != want {
            text.0 = want.to_string();
            color.0 = mode_chip_color(ime.enabled());
        }
        tf.translation.x = cx;
        tf.translation.y = -cy;
        *v = Visibility::Visible;
    }
}

// ----------------------------------------------------------------------------
// Plugin
// ----------------------------------------------------------------------------

pub struct PinyinImePlugin;

impl Plugin for PinyinImePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PinyinIme::new());
        app.init_resource::<ImeFocus>();
        app.add_systems(PreUpdate, (pinyin_ime_system, clear_ime_focus).chain());
        // 候选条必须在 Update 所有输入框回填 ImeFocus 之后渲染；否则聚焦框尚未设置，候选条不会显示。
        app.add_systems(PostUpdate, pinyin_candidate_ui_system);
        // 模式 chip 同契约（PostUpdate、读 Update 回填的 ImeFocus）
        app.add_systems(PostUpdate, pinyin_mode_chip_system);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;

    /// 单个字符键（按下）
    fn char_key(ch: &str) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::Space, // 逻辑只看 logical_key，物理码随意
            logical_key: Key::Character(ch.into()),
            state: ButtonState::Pressed,
            text: Some(ch.into()),
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    /// Shift 键（按下/松开）
    fn shift_key(state: ButtonState) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::ShiftLeft,
            logical_key: Key::Shift,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    fn send(app: &mut App, ev: KeyboardInput) {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(ev);
    }

    fn focus(app: &mut App, rect: Option<(f32, f32, f32, f32)>) {
        app.world_mut().resource_mut::<ImeFocus>().rect = rect;
    }

    // ---- 纯逻辑 ----

    #[test]
    fn engine_lookup_and_commit() {
        let mut ime = PinyinIme::new();
        ime.toggle(); // 中文模式
        assert!(ime.enabled());
        for c in "nihao".chars() {
            ime.feed_letter(c);
        }
        assert_eq!(ime.composing, "nihao");
        assert_eq!(ime.candidates.first(), Some(&"你好".to_string()));
        ime.commit_default(); // 空格/Enter 选首个
        assert_eq!(ime.take_commit(), Some("你好".to_string()));
        assert!(!ime.is_composing());
    }

    #[test]
    fn engine_single_char_select() {
        let mut ime = PinyinIme::new();
        ime.toggle();
        for c in "hao".chars() {
            ime.feed_letter(c);
        }
        // 选第 2 个候选（libpinyin "hao" 首位为「号」，次位为「好」）
        let want = ime.candidates.get(1).cloned().expect("hao 至少 2 个候选");
        ime.select(1);
        assert_eq!(ime.take_commit(), Some(want));
    }


    /// consumes_key：英文模式不接管任何键；中文模式接管字母（组合中）
    #[test]
    fn consumes_key_contract() {
        let mut ime = PinyinIme::new();
        let pressed = char_key("a");
        assert!(!ime.consumes_key(&pressed)); // 英文模式
        ime.toggle();
        assert!(!ime.consumes_key(&pressed)); // 中文但未组合 → 不接管字母（首字母由 PreUpdate 喂入后才开始接管）
        ime.feed_letter('a');
        // 组合中：再按字母应被接管
        assert!(ime.consumes_key(&char_key("b")));
    }

    // ---- 集成（真实 bevy 调度）----

    #[test]
    fn integration_shift_toggle_and_pinyin_pipeline() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        // 候选条 UI 系统读 Res<UiFont>，需提供（弱句柄 → 不 spawn 实体，逻辑测试足够）
        app.insert_resource(crate::ui::sprite_ui::UiFont(Handle::<Font>::default()));

        // 1) Shift 单按切中文（无需聚焦）
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();
        assert!(app.world().resource::<PinyinIme>().enabled());

        // 2) 喂 n-i-h-a-o（每帧重设 focus.rect，PreUpdate 末尾会被清空）
        for c in "nihao".chars() {
            focus(&mut app, Some((10.0, 10.0, 100.0, 16.0)));
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        {
            let ime = app.world().resource::<PinyinIme>();
            assert_eq!(ime.composing, "nihao");
            assert!(ime.candidates.iter().any(|c| c == "你好"));
        }

        // 3) 数字 1 选首个候选 → 提交 你好
        focus(&mut app, Some((10.0, 10.0, 100.0, 16.0)));
        send(&mut app, char_key("1"));
        app.update();

        let mut ime = app.world_mut().resource_mut::<PinyinIme>();
        assert_eq!(ime.take_commit(), Some("你好".to_string()));
        assert!(!ime.is_composing());
    }

    /// 集成：无聚焦框时字母不进 IME（不会误吞英文输入）
    #[test]
    fn integration_no_focus_does_not_consume() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.insert_resource(crate::ui::sprite_ui::UiFont(Handle::<Font>::default()));

        // 切中文
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();

        // 不设 focus.rect，喂字母
        send(&mut app, char_key("a"));
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(!ime.is_composing()); // 字母未被吞入拼音缓冲
    }

    /// libpinyin 整句组合：连续拼音首候选为整句（nihaomawojiao → 你好吗我叫）。
    #[test]
    fn libpinyin_sentence_composition() {
        let mut ime = PinyinIme::new();
        ime.toggle();
        for c in "nihaomawojiao".chars() {
            ime.feed_letter(c);
        }
        assert_eq!(ime.candidates.first().map(String::as_str), Some("你好吗我叫"));
    }

    /// libpinyin 候选含常用字/词（hao→好、jintian→今天、nv→女、lv 有候选）。
    #[test]
    fn libpinyin_candidates_contain_common() {
        let mut ime = PinyinIme::new();
        ime.toggle();
        let check = |ime: &mut PinyinIme, py: &str, want: &str| {
            for c in py.chars() {
                ime.feed_letter(c);
            }
            assert!(ime.candidates.iter().any(|c| c == want), "{} 候选中应含「{}」，实际 {:?}", py, want, ime.candidates);
            ime.cancel();
            ime.toggle(); // cancel 后仍中文？toggle 切回中文
            ime.toggle(); // 回到中文
        };
        check(&mut ime, "hao", "好");
        check(&mut ime, "jintian", "今天");
        check(&mut ime, "nv", "女");
        check(&mut ime, "lv", "旅");
    }

    /// 选首位整句候选 → 上屏整句。
    #[test]
    fn libpinyin_commit_sentence() {
        let mut ime = PinyinIme::new();
        ime.toggle();
        for c in "nihaomawojiao".chars() {
            ime.feed_letter(c);
        }
        ime.select(0); // 选首位“你好吗我叫”
        assert_eq!(ime.take_commit(), Some("你好吗我叫".to_string()));
        assert!(!ime.is_composing());
    }

    /// 审查 B2：部分选词后继续输入——选中「你好」后剩余 "ma" 续打，候选/上屏不脱节。
    #[test]
    fn libpinyin_partial_select_then_continue() {
        let mut ime = PinyinIme::new();
        ime.toggle();
        for c in "nihaoma".chars() {
            ime.feed_letter(c);
        }
        let idx = ime
            .candidates
            .iter()
            .position(|c| c == "你好")
            .expect("nihaoma 候选中应含「你好」");
        ime.select(idx);
        // 选中词立即上屏
        assert_eq!(ime.take_commit(), Some("你好".to_string()));
        // 剩余拼音 "ma" 继续组合
        assert_eq!(ime.composing, "ma");
        assert!(ime.candidates.iter().any(|c| c == "吗"), "剩余 ma 候选应含「吗」，实际 {:?}", ime.candidates);
        // 继续输入 wojiao → mawojiao
        for c in "wojiao".chars() {
            ime.feed_letter(c);
        }
        assert_eq!(ime.composing, "mawojiao");
        assert!(ime.candidates.iter().any(|c| c == "吗我叫"), "mawojiao 候选应含「吗我叫」，实际 {:?}", ime.candidates);
    }


    /// 空组合回归：无输入时候选为空且不 panic。
    #[test]
    fn empty_composing_no_panic() {
        let mut ime = PinyinIme::new();
        ime.toggle();
        assert!(!ime.is_composing());
        assert!(ime.candidates.is_empty());
        assert!(ime.page_candidates().is_empty());
    }

    /// Backspace 删尽组合后 recompute 不再 panic，状态回到未组合。
    #[test]
    fn backspace_to_empty_no_panic() {
        let mut ime = PinyinIme::new();
        ime.toggle(); // 中文模式
        ime.feed_letter('n');
        ime.feed_letter('i');
        assert!(ime.is_composing());
        assert!(!ime.candidates.is_empty(), "前置：音节 ni 应有候选（后续复位断言才有区分度）");
        ime.backspace(); // 删到 "n"
        assert!(ime.is_composing());
        ime.backspace(); // 删到空 → 旧实现此处 panic
        assert!(!ime.is_composing());
        assert!(ime.candidates.is_empty());
    }

    // ---- 中/英模式 chip ----

    #[test]
    fn mode_chip_text_and_pos() {
        assert_eq!(mode_chip_text(true), "中");
        assert_eq!(mode_chip_text(false), "英");
        // 常规：框右侧 + 4px、垂直居中（12px 字高的一半）
        assert_eq!(mode_chip_pos(100.0, 200.0, 120.0, 16.0), (224.0, 202.0));
        // 贴右缘：内缩进 1024-14=1010，不出画布
        assert_eq!(mode_chip_pos(1000.0, 10.0, 30.0, 16.0), (1010.0, 12.0));
    }

    /// chip 行为：默认英、聚焦才显示、Shift 翻转文本+颜色（用户可发现性）。
    /// 强字体句柄走真实 spawn 路径；Update 阶段回填 focus（对齐生产契约）。
    #[test]
    fn mode_chip_shows_and_flips() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.add_plugins(bevy::asset::AssetPlugin::default()); // Assets::add 需 AssetServer
        app.init_asset::<Font>();
        // 强句柄：内置 TTF（chip/候选条 spawn 都要求 is_strong）
        let bytes = include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let font = Font::from_bytes(bytes.to_vec());
        let handle = app.world_mut().resource_mut::<Assets<Font>>().add(font);
        app.insert_resource(crate::ui::sprite_ui::UiFont(handle));
        // 生产契约：Update 阶段由各文本框回填 ImeFocus（PreUpdate 会被 clear_ime_focus 清掉）
        app.add_systems(Update, |mut f: ResMut<ImeFocus>| {
            f.rect = Some((100.0, 200.0, 120.0, 16.0));
        });

        // 1) 首帧 spawn（Commands 下一帧生效）、次帧设文本（默认英文模式）
        app.update();
        app.update();
        {
            let mut q = app
                .world_mut()
                .query_filtered::<(&Text2d, &Visibility), With<PinyinModeChip>>();
            let (t, v) = q.single(app.world()).expect("字体强句柄后 chip 已 spawn");
            assert_eq!(t.0, "英");
            assert_eq!(*v, Visibility::Visible);
        }

        // 2) Shift 单按 → 中文：文本与颜色同帧翻转
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();
        {
            let mut q = app
                .world_mut()
                .query_filtered::<(&Text2d, &TextColor, &Transform), With<PinyinModeChip>>();
            let (t, c, tf) = q.single(app.world()).unwrap();
            assert_eq!(t.0, "中");
            let Color::Srgba(s) = c.0 else {
                panic!("srgb 颜色")
            };
            assert!((s.red - 1.0).abs() < 1e-3 && (s.blue - 0.3).abs() < 1e-3);
            // 位置：框右 (100+120+4, -(200+8-6))
            assert_eq!(tf.translation.x, 224.0);
            assert_eq!(tf.translation.y, -202.0);
        }

        // 3) 仍聚焦：保持显示（隐藏路径由 mode_chip_hidden_without_focus 覆盖）
        let mut q = app
            .world_mut()
            .query_filtered::<&Visibility, With<PinyinModeChip>>();
        assert_eq!(*q.single(app.world()).unwrap(), Visibility::Visible);
    }

    /// 无聚焦帧：chip 隐藏（spawn 存在性由上一测试覆盖，这里弱句柄也行——不 spawn 即无实体，
    /// 故同样用强句柄确保实体存在后走 focus=None 分支）
    #[test]
    fn mode_chip_hidden_without_focus() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.add_plugins(bevy::asset::AssetPlugin::default()); // Assets::add 需 AssetServer
        app.init_asset::<Font>();
        let bytes = include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let font = Font::from_bytes(bytes.to_vec());
        let handle = app.world_mut().resource_mut::<Assets<Font>>().add(font);
        app.insert_resource(crate::ui::sprite_ui::UiFont(handle));

        app.update(); // spawn（Hidden）
        app.update(); // 无回填 → focus=None 路径
        let mut q = app
            .world_mut()
            .query_filtered::<&Visibility, With<PinyinModeChip>>();
        assert_eq!(*q.single(app.world()).unwrap(), Visibility::Hidden);
    }

    // ---- 候选条失焦滞留（候选条可见性失焦帧落地的回归）----

    /// Update 阶段可控回填 ImeFocus（对齐生产契约：文本框每帧 Update 重写 focus.rect）
    #[derive(Resource, Default)]
    struct BackfillFocus(bool);

    /// 自绘候选窗：Shift 切中文 + 聚焦 + 输入 nihao → 候选条渲染出“拼音 + 候选”文本。
    /// 这是对“游戏内候选窗口”的硬证明：候选条实体(PinyinBarText)可见且内容正确。
    #[test]
    fn candidate_bar_renders_pinyin_and_candidates() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Font>();
        let bytes = include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let handle = app.world_mut().resource_mut::<Assets<Font>>().add(Font::from_bytes(bytes.to_vec()));
        app.insert_resource(crate::ui::sprite_ui::UiFont(handle));
        // Update 阶段回填 focus（对齐生产契约：各文本框每帧 Update 回填 ImeFocus）
        app.add_systems(Update, |mut f: ResMut<ImeFocus>| {
            f.rect = Some((10.0, 10.0, 100.0, 16.0));
        });
        // 1) Shift 单按切中文
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();
        assert!(app.world().resource::<PinyinIme>().enabled());
        // 2) 输入 n-i-h-a-o（每帧喂一个字母，候选条实体下一帧可见）
        for c in "nihao".chars() {
            focus(&mut app, Some((10.0, 10.0, 100.0, 16.0)));
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        app.update();
        // 3) 候选条文本应包含拼音串与候选「你好」
        let mut q = app
            .world_mut()
            .query_filtered::<&Text2d, With<PinyinBarText>>();
        let bar_text = q.single(app.world()).unwrap().0.clone();
        assert!(bar_text.contains("nihao"), "候选条应含拼音 nihao，实际: {:?}", bar_text);
        assert!(bar_text.contains("你好"), "候选条应含候选「你好」，实际: {:?}", bar_text);
        // 4) 候选条可见
        let mut vq = app
            .world_mut()
            .query_filtered::<&Visibility, With<PinyinBarText>>();
        assert_eq!(*vq.single(app.world()).unwrap(), Visibility::Visible);
    }

    /// 组合中途失焦（focus=None）当帧候选条必须落回 Hidden：
    /// 旧实现可见性只写在 `if let Some(focus.rect)` 内，失焦帧算出的 Hidden 永不落地，
    /// 候选条带着旧组合（"ni 1.你 …"）无限悬浮在失焦的界面上（对齐 mode chip 测法）。
    #[test]
    fn candidate_bar_hidden_without_focus() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.add_plugins(bevy::asset::AssetPlugin::default()); // Assets::add 需 AssetServer
        app.init_asset::<Font>();
        // 强句柄：候选条 spawn 要求 is_strong（对齐 mode chip 测试）
        let bytes = include_bytes!("../../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let font = Font::from_bytes(bytes.to_vec());
        let handle = app.world_mut().resource_mut::<Assets<Font>>().add(font);
        app.insert_resource(crate::ui::sprite_ui::UiFont(handle));
        app.insert_resource(BackfillFocus(true));
        app.add_systems(
            Update,
            |flag: Res<BackfillFocus>, mut f: ResMut<ImeFocus>| {
                if flag.0 {
                    f.rect = Some((10.0, 10.0, 100.0, 16.0));
                }
            },
        );

        // 切中文（Shift 单按，无需聚焦）
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();

        // 聚焦 + 喂 n,i → 组合中：候选条应 Visible（spawn 的 Commands 下一帧生效）
        send(&mut app, char_key("n"));
        app.update();
        send(&mut app, char_key("i"));
        app.update();
        app.update();
        let mut bg = app
            .world_mut()
            .query_filtered::<&Visibility, With<PinyinBarBg>>();
        let mut txt = app
            .world_mut()
            .query_filtered::<&Visibility, With<PinyinBarText>>();
        assert!(app.world().resource::<PinyinIme>().is_composing());
        assert_eq!(*bg.single(app.world()).unwrap(), Visibility::Visible);
        assert_eq!(*txt.single(app.world()).unwrap(), Visibility::Visible);

        // 失焦帧（组合仍在）：候选条当帧必须落回 Hidden
        app.world_mut().resource_mut::<BackfillFocus>().0 = false;
        app.update();
        // 组合未被取消——上面的 Hidden 只能来自失焦路径，而非组合结束
        assert!(app.world().resource::<PinyinIme>().is_composing());
        assert_eq!(*bg.single(app.world()).unwrap(), Visibility::Hidden);
        assert_eq!(*txt.single(app.world()).unwrap(), Visibility::Hidden);
    }
}
