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
//     接管则跳过；再 ime.take_commit() 注入已选汉字；并回填 ImeFocus（聚焦框矩形）。
// ============================================================================

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use std::collections::{HashMap, HashSet};

use crate::ui::sprite_ui::{UiEntity, UiFont};

/// 候选条每页数量
const CANDS_PER_PAGE: usize = 9;

// ----------------------------------------------------------------------------
// 词典（assets/data/pinyin_dict.txt 编译期 include_bytes! 内嵌；一次性脚本生成后签入，见 LICENSE-unicode.txt）
// ----------------------------------------------------------------------------

/// 内嵌完整词典（编译期 include_bytes!）。
const PINYIN_DICT_BYTES: &[u8] = include_bytes!("../../assets/data/pinyin_dict.txt");

/// 拼音→汉字/词 词典 + 音节切分。
/// - words: 多字词（人工策展），精确匹配优先。
/// - chars: 单音节→单字候选（按字频排序，来自 Unicode Unihan kHanyuPinlu）。
/// - syllables: 有效拼音音节集合（= chars 的键），用于切分输入串。
#[derive(Resource)]
pub struct PinyinDict {
    words: HashMap<String, Vec<String>>,
    chars: HashMap<String, Vec<char>>,
    syllables: HashSet<String>,
}

impl PinyinDict {
    /// 极小内嵌词典：仅单元测试用（不读文件）。完整词典见 [`PinyinDict::load`]。
    #[cfg(test)]
    fn minimal() -> Self {
        // (拼音, [候选...])
        const ENTRIES: &[(&str, &[&str])] = &[
            // ---- 常用词 ----
            ("nihao", &["你好"]),
            ("xiexie", &["谢谢"]),
            ("zaijian", &["再见"]),
            ("duibuqi", &["对不起"]),
            ("meiguanxi", &["没关系"]),
            ("wohenhao", &["我很好"]),
            ("shime", &["什么"]),
            ("weisheme", &["为什么"]),
            ("zhidao", &["知道"]),
            ("xiexie", &["谢谢"]),
            ("pengyou", &["朋友"]),
            ("mingbai", &["明白"]),
            ("yiqi", &["一起"]),
            ("dengdai", &["等待"]),
            // ---- 单字（常用音节，按常用度粗排）----
            ("de", &["的", "得", "地"]),
            ("shi", &["是", "时", "事", "十", "使"]),
            ("ni", &["你", "泥", "尼", "拟"]),
            ("wo", &["我", "窝", "握"]),
            ("ta", &["他", "她", "它", "塔"]),
            ("zhe", &["这", "着", "者"]),
            ("ge", &["个", "各", "哥"]),
            ("li", &["里", "理", "力", "立", "李"]),
            ("zai", &["在", "再", "栽"]),
            ("ren", &["人", "认", "任"]),
            ("you", &["有", "又", "右", "友"]),
            ("hao", &["好", "号", "豪"]),
            ("ma", &["吗", "妈", "马", "麻"]),
            ("ba", &["吧", "把", "八", "巴"]),
            ("de", &["的", "得", "地"]),
            ("bu", &["不", "步", "部"]),
            ("ke", &["可", "课", "克", "客"]),
            ("yi", &["一", "以", "已", "意", "易"]),
            ("da", &["大", "打", "达"]),
            ("shang", &["上", "商", "伤"]),
            ("xia", &["下", "夏", "吓"]),
            ("zhong", &["中", "种", "重", "钟"]),
            ("guo", &["国", "过", "果"]),
            ("shuo", &["说", "硕"]),
            ("lai", &["来", "赖"]),
            ("qu", &["去", "区", "曲"]),
            ("dui", &["对", "队", "堆"]),
            ("mei", &["没", "美", "梅", "每"]),
            ("kan", &["看", "刊"]),
            ("xiang", &["想", "向", "像", "项"]),
            ("jiao", &["叫", "教", "角", "脚"]),
            ("xian", &["现", "先", "线", "显"]),
            ("na", &["那", "拿", "哪"]),
            ("ji", &["几", "机", "级", "集", "记"]),
            ("tian", &["天", "田", "添"]),
            ("xin", &["心", "新", "信", "辛"]),
            ("jia", &["家", "加", "假", "价"]),
            ("deng", &["等", "灯", "登"]),
            ("gong", &["工", "公", "共", "功"]),
            ("hui", &["会", "回", "灰", "惠"]),
            ("dao", &["到", "道", "岛", "倒"]),
            ("le", &["了", "乐", "勒"]),
            ("men", &["们", "门"]),
            ("xue", &["学", "雪", "血"]),
            ("sheng", &["生", "声", "升", "省"]),
            ("jian", &["见", "间", "建", "剑"]),
            ("shi", &["是", "时", "事", "十"]),
            ("wang", &["王", "往", "望", "网"]),
            ("chang", &["长", "场", "常", "唱"]),
            ("qian", &["前", "钱", "千", "浅"]),
            ("hou", &["后", "候", "厚"]),
            ("zuo", &["做", "作", "坐", "左"]),
            ("shi", &["是", "时", "事"]),
            ("fa", &["发", "法", "罚"]),
            ("jie", &["接", "节", "结", "姐", "界"]),
            ("ci", &["次", "此", "词", "刺"]),
            ("dian", &["点", "电", "店", "典"]),
            ("bian", &["边", "变", "便", "编"]),
            ("wen", &["问", "文", "温"]),
            ("hua", &["话", "花", "画", "华"]),
            ("ming", &["名", "明", "命", "鸣"]),
            ("zi", &["字", "子", "自", "资"]),
            ("dong", &["动", "东", "洞", "懂"]),
            ("cheng", &["成", "城", "程", "承"]),
        ];
        // 多字候选 → words；单字候选 → chars（minimal 仅测试用）
        let mut words: HashMap<String, Vec<String>> = HashMap::new();
        let mut chars: HashMap<String, Vec<char>> = HashMap::new();
        for (py, cands) in ENTRIES {
            if cands.iter().any(|c| c.chars().count() > 1) {
                let e = words.entry((*py).to_string()).or_default();
                for c in *cands {
                    if !e.iter().any(|x| x == c) {
                        e.push((*c).to_string());
                    }
                }
            } else {
                let e = chars.entry((*py).to_string()).or_default();
                for c in *cands {
                    for ch in c.chars() {
                        if !e.contains(&ch) {
                            e.push(ch);
                        }
                    }
                }
            }
        }
        let syllables = chars.keys().cloned().collect();
        Self {
            words,
            chars,
            syllables,
        }
    }

    /// 从内嵌文件加载完整词典（启动期一次性解析，~150KB 文本很快）。
    pub fn load() -> Self {
        Self::from_text(PINYIN_DICT_BYTES)
    }

    fn from_text(bytes: &[u8]) -> Self {
        let text = std::str::from_utf8(bytes).unwrap_or("");
        let mut words: HashMap<String, Vec<String>> = HashMap::new();
        let mut chars: HashMap<String, Vec<char>> = HashMap::new();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, val)) = line.split_once('\t') else {
                continue;
            };
            if let Some(py) = key.strip_prefix('@') {
                if !val.is_empty() {
                    words.entry(py.to_string()).or_default().push(val.to_string());
                }
            } else if let Some(py) = key.strip_prefix('$') {
                let v: Vec<char> = val.chars().collect();
                if !v.is_empty() {
                    chars.entry(py.to_string()).or_default().extend(v);
                }
            }
        }
        let syllables = chars.keys().cloned().collect();
        Self {
            words,
            chars,
            syllables,
        }
    }

    /// 把输入串切成有效音节序列（贪心最长匹配）；无法整串切分返回 None。
    /// 输入假定纯 ASCII 小写（来自 feed_letter）。
    fn segment<'s>(&self, s: &'s str) -> Option<Vec<&'s str>> {
        let bytes = s.as_bytes();
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            let max_len = (bytes.len() - pos).min(6);
            let mut found_len = None;
            for len in (1..=max_len).rev() {
                if let Ok(sub) = std::str::from_utf8(&bytes[pos..pos + len]) {
                    if self.syllables.contains(sub) {
                        found_len = Some(len);
                        break;
                    }
                }
            }
            match found_len {
                Some(len) => {
                    let sub = std::str::from_utf8(&bytes[pos..pos + len]).unwrap();
                    out.push(sub);
                    pos += len;
                }
                None => return None,
            }
        }
        Some(out)
    }

    /// 查候选：① 精确词 ② 单音节→完整单字列表 ③ 多音节→各音节 top1 拼接 + 末音节变体。
    fn lookup(&self, composing: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        // ① 词
        if let Some(ws) = self.words.get(composing) {
            out.extend(ws.clone());
        }
        // ②/③ 切分
        if let Some(syls) = self.segment(composing) {
            if syls.iter().all(|s| self.chars.contains_key(*s)) {
                if syls.len() == 1 {
                    if let Some(cs) = self.chars.get(syls[0]) {
                        out.extend(cs.iter().map(|c| c.to_string()));
                    }
                } else {
                    // 主候选：各音节 top1 拼接
                    let primary: String = syls
                        .iter()
                        .filter_map(|s| self.chars.get(*s).and_then(|cs| cs.first()).copied())
                        .collect();
                    out.push(primary);
                    // 末音节轮换 top-N，提供短语变体
                    let (head_syls, last) = syls.split_at(syls.len() - 1);
                    let head: String = head_syls
                        .iter()
                        .filter_map(|s| self.chars.get(*s).and_then(|cs| cs.first()).copied())
                        .collect();
                    if let Some(cs) = self.chars.get(last[0]) {
                        for &c in cs.iter().take(7).skip(1) {
                            let mut v = head.clone();
                            v.push(c);
                            out.push(v);
                        }
                    }
                }
            }
        }
        // 去重保序
        let mut seen = HashSet::new();
        out.retain(|s| seen.insert(s.clone()));
        out
    }
}

// ----------------------------------------------------------------------------
// IME 状态资源
// ----------------------------------------------------------------------------

#[derive(Resource)]
pub struct PinyinIme {
    /// 中/英模式（false=英文，沿用原有行为）
    enabled: bool,
    /// 当前拼音缓冲（如 "nihao"）
    composing: String,
    /// 当前候选列表
    candidates: Vec<String>,
    /// 候选分页
    page: usize,
    /// 本帧要插入聚焦框的已选文本
    commit_pending: Option<String>,
    /// 本帧 IME 用退格编辑过拼音（供文本框判断是否跳过删除缓冲）
    ate_edit: bool,
    dict: PinyinDict,
}

impl PinyinIme {
    pub fn new(dict: PinyinDict) -> Self {
        Self {
            enabled: false,
            composing: String::new(),
            candidates: Vec::new(),
            page: 0,
            commit_pending: None,
            ate_edit: false,
            dict,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_composing(&self) -> bool {
        !self.composing.is_empty()
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
    }

    fn feed_letter(&mut self, c: char) {
        self.composing.push(c);
        self.page = 0;
        self.recompute();
    }

    fn backspace(&mut self) {
        if self.composing.pop().is_some() {
            self.ate_edit = true;
            self.page = 0;
            self.recompute();
        }
    }

    fn select(&mut self, idx: usize) {
        let abs = self.page * CANDS_PER_PAGE + idx;
        if let Some(c) = self.candidates.get(abs).cloned() {
            self.commit(c);
        }
    }

    /// 空格 / Enter（组合中）：选首个候选；无候选则把原始拼音作为 ASCII 提交（逃生口）
    fn commit_default(&mut self) {
        if !self.is_composing() {
            return;
        }
        if let Some(c) = self.candidates.first().cloned() {
            self.commit(c);
        } else {
            // 词典无匹配：上屏原始字母，避免用户输入卡死
            let raw = std::mem::take(&mut self.composing);
            self.commit(raw);
        }
    }

    fn commit(&mut self, s: String) {
        // 多次提交合并（一帧内罕见）
        if let Some(existing) = &mut self.commit_pending {
            existing.push_str(&s);
        } else {
            self.commit_pending = Some(s);
        }
        self.composing.clear();
        self.candidates.clear();
        self.page = 0;
    }

    fn cancel(&mut self) {
        self.composing.clear();
        self.candidates.clear();
        self.page = 0;
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
        self.candidates = self.dict.lookup(&self.composing);
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
        let acting = self.is_composing() || self.commit_pending.is_some() || self.ate_edit;
        match &key.logical_key {
            Key::Character(c) => {
                let s = c.as_str();
                if s.chars().all(|ch| ch.is_ascii_alphabetic()) {
                    // 字母：IME 正在组合时接管（PreUpdate 已把它喂进拼音）
                    self.is_composing()
                } else if s.chars().all(|ch| ch.is_ascii_digit()) {
                    acting // 数字选候选（仅组合中）
                } else if (s == "-" || s == "=") && self.is_composing() {
                    true // 翻页键（仅组合中）
                } else {
                    false
                }
            }
            Key::Space => acting,
            Key::Backspace => self.is_composing() || self.ate_edit,
            Key::Enter => acting,
            Key::Escape => self.is_composing(),
            _ => false,
        }
    }
}

// ----------------------------------------------------------------------------
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
                    tracing::info!("[IME] 切换: {}", if ime.enabled() { "中文" } else { "英文" });
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
                    // 数字选候选（仅组合中）
                    if ime.is_composing() {
                        if let Ok(n) = s.parse::<usize>() {
                            if n >= 1 {
                                ime.select(n - 1);
                            }
                        }
                    }
                } else if s == "-" {
                    if ime.is_composing() {
                        ime.page_prev();
                    }
                } else if s == "=" {
                    if ime.is_composing() {
                        ime.page_next();
                    }
                }
            }
            Key::Space => {
                if ime.is_composing() {
                    ime.commit_default();
                }
            }
            Key::Enter => {
                if ime.is_composing() {
                    ime.commit_default();
                }
            }
            Key::Backspace => {
                if ime.is_composing() {
                    ime.backspace();
                }
            }
            Key::Escape => {
                if ime.is_composing() {
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

    // 决定是否显示：中文模式 + 正在组合 + 有聚焦框
    let show = ime.enabled() && ime.is_composing() && focus.rect.is_some();

    // 文本：拼音串 + 候选 1.xxx 2.xxx ...
    let mut label = ime.composing.clone();
    if !ime.page_candidates().is_empty() {
        label.push_str("  ");
        for (i, c) in ime.page_candidates().iter().enumerate() {
            label.push_str(&format!("{}.{} ", i + 1, c));
        }
    }

    let vis = if show { Visibility::Visible } else { Visibility::Hidden };

    if let Some((x, y, _w, h)) = focus.rect {
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
}

// ----------------------------------------------------------------------------
// Plugin
// ----------------------------------------------------------------------------

pub struct PinyinImePlugin;

impl Plugin for PinyinImePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PinyinIme::new(PinyinDict::load()));
        app.init_resource::<ImeFocus>();
        app.add_systems(PreUpdate, (pinyin_ime_system, clear_ime_focus).chain());
        app.add_systems(Update, pinyin_candidate_ui_system);
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
        let mut ime = PinyinIme::new(PinyinDict::minimal());
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
        let mut ime = PinyinIme::new(PinyinDict::minimal());
        ime.toggle();
        for c in "hao".chars() {
            ime.feed_letter(c);
        }
        // 选第 2 个候选（"号"）
        ime.select(1);
        assert_eq!(ime.take_commit(), Some("号".to_string()));
    }

    /// 逃生口：词典无匹配时上屏原始字母（不会卡死输入）
    #[test]
    fn engine_fallback_raw() {
        let mut ime = PinyinIme::new(PinyinDict::minimal());
        ime.toggle();
        for c in "zzz".chars() {
            ime.feed_letter(c);
        }
        ime.commit_default();
        assert_eq!(ime.take_commit(), Some("zzz".to_string()));
    }

    /// consumes_key：英文模式不接管任何键；中文模式接管字母（组合中）
    #[test]
    fn consumes_key_contract() {
        let mut ime = PinyinIme::new(PinyinDict::minimal());
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

    /// 完整内嵌词典加载 + 高频字排首位（验证 Unihan 字频排序生效）。
    #[test]
    fn full_dict_loads_and_orders() {
        let d = PinyinDict::load();
        let first = |py: &str| d.chars.get(py).and_then(|cs| cs.first()).copied();
        assert_eq!(first("de"), Some('的'));
        assert_eq!(first("shi"), Some('是'));
        assert_eq!(first("hao"), Some('好'));
        assert_eq!(first("ni"), Some('你'));
        assert_eq!(first("zhe"), Some('这'));
        // ü→v 键盘约定：nv=女、lv 有候选
        assert_eq!(first("nv"), Some('女'));
        assert!(first("lv").is_some());
    }

    /// 音节切分：多音节无词项也能拼出短语；贪心最长不误切。
    #[test]
    fn segmentation_joins_syllables() {
        let d = PinyinDict::load();
        // zhege 无词项，靠切分 ni+hao 那样的拼接得「这个」
        let cands = d.lookup("zhege");
        assert!(
            cands.iter().any(|c| c == "这个"),
            "zhege 候选应含「这个」，实际: {:?}",
            cands
        );
        // 贪心最长：xian 是一个音节，不应切成 xi+an
        assert_eq!(d.segment("xian"), Some(vec!["xian"]));
        assert_eq!(d.segment("nihao"), Some(vec!["ni", "hao"]));
        // 无效串切不出来
        assert_eq!(d.segment("zzz"), None);
    }
}
