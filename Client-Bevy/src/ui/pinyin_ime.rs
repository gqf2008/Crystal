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
// 与各文本框系统的契约（#2596 统一化）：
//   - pinyin_ime_system 跑在 PreUpdate，逐事件处理 KeyboardInput：真正被 IME
//     用掉的按键记入「本帧消费表」；组合/提交/编辑标记不再作为吞键依据。
//   - 各文本框系统跑在 Update：先 ime.unconsumed(&key_list) 取本帧事件批中
//     未被 IME 用掉的事件再逐个处理（同值重复按键按 IME 实际用掉的次数配给
//     ——组合 "n" 时同帧两个退格：第一个删拼音，第二个照常删正文）；再
//     ime.take_commit() 注入已选汉字（提交只在产生帧有效，下一帧 PreUpdate
//     未被取走即过期丢弃）；并回填 ImeFocus（聚焦框矩形，须每帧重写——
//     PreUpdate 会被 clear_ime_focus 清空，而候选条失焦帧即隐藏，只在聚焦
//     变化时写一次的框会让候选条在聚焦期间闪烁/消失）。
//   - ButtonInput 类消费方（just_pressed 无法区分事件实例）用
//     ime.escape_consumed() 让路：取消组合的那次 Esc 已被 IME 用掉，当帧
//     不再触发关对话框/关菜单。
//   - 失焦即弃组：组合中途失焦（如点进密码框），下一帧组合自动取消——
//     组合不会跨失焦冻结存活去吞后续按键。
//   - 中文模式组合为空时 Shift+大写字母直通文本框并进入「临时英文段」：
//     段内所有按键直通（中文模式也能整体打出 Bob / A@B.com），空格/回车/
//     Esc 结束段后自动回中文（搜狗「Shift 切英文段」的单段节奏）；
//     Shift 单按切换中/英（OS 自动重复的 Shift 不重新武装单按判定）。
//   - pinyin_mode_chip_system 跑在 PostUpdate：聚焦框右侧常显「中/英」模式 chip。
//     系统 IME 被禁用后 OS 输入法工具条不再出现——chip 是它的游戏内等价物
//     （手心/搜狗用户的「中/英指示 + Shift 切换」通用约定），否则内置 IME 无从发现。
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
                    words
                        .entry(py.to_string())
                        .or_default()
                        .push(val.to_string());
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
    /// 不变式：Some 结果必含 ≥1 个音节——空串返回 None 而非 Some(vec![])，
    /// 否则空表会让调用方的 `syls.len() - 1` 类索引 usize 下溢 panic（#2594 曾因此崩溃）。
    fn segment<'s>(&self, s: &'s str) -> Option<Vec<&'s str>> {
        if s.is_empty() {
            return None;
        }
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
        // 出口钉死不变式（Some ⇒ ≥1 音节）：违反时在此炸出，
        // 而不是等调用方 `syls.len() - 1` 类索引下溢才崩（#2594 类 panic 的根因位）
        debug_assert!(!out.is_empty());
        Some(out)
    }

    /// 查候选：① 精确词 ② 单音节→完整单字列表 ③ 多音节→各音节 top1 拼接 + 末音节变体。
    /// 空组合（Backspace 删尽后 recompute）直接返回空——曾因 segment("") 切出空音节表，
    /// ③ 的 `syls.len() - 1` usize 下溢 panic（登录界面输入字母再退格可稳定复现）；
    /// 如今 segment 对空串也返回 None（根因修复），此处守卫保留作双保险。
    fn lookup(&self, composing: &str) -> Vec<String> {
        if composing.is_empty() {
            return Vec::new();
        }
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
    /// 本帧要插入聚焦框的已选文本（只在产生帧有效：下一帧 PreUpdate 未被
    /// take_commit 取走即过期丢弃——滞留的提交会在之后任意获焦框意外注入）
    commit_pending: Option<String>,
    /// 本帧被 IME 真正消费的按键事件（逐事件记录）。unconsumed 据此配给，
    /// 取代旧的「组合中/有提交/刚编辑」整体吞键——那会把同帧后段未被 IME
    /// 处理的键也吞掉（如退格删尽组合后同帧的空格静默丢失）
    consumed: Vec<KeyboardInput>,
    /// 本帧是否有被消费的 Esc（取消组合的那次）。ButtonInput 类消费方
    /// （just_pressed 只知帧级真假、无从区分事件实例）据此让路。
    consumed_escape: bool,
    /// 临时英文段（Shift+大写进入）：段内所有按键直通文本框、不进拼音，
    /// 空格/回车/Esc 结束段。中文模式可整体打出 'Bob'/'A@B.com'（搜狗
    /// 「Shift 切英文段」的单段节奏——只放行单个大写会让 'Bob' 的 'o'
    /// 又进拼音组合）
    temp_english: bool,
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
            consumed: Vec::new(),
            consumed_escape: false,
            temp_english: false,
            dict,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_composing(&self) -> bool {
        !self.composing.is_empty()
    }

    /// 当前拼音缓冲（e2e 真值：GetState 报告字母是否到达 IME）
    pub fn composing_text(&self) -> &str {
        &self.composing
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
        // 显式中/英切换即终止临时英文段（段跟着中文模式走）
        self.temp_english = false;
    }

    fn feed_letter(&mut self, c: char) {
        self.composing.push(c);
        self.page = 0;
        self.recompute();
    }

    fn backspace(&mut self) {
        if self.composing.pop().is_some() {
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
        self.temp_english = false;
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

    /// 本帧事件批中未被 IME 用掉的事件（文本框系统逐个处理这些）。
    /// 同值重复按键（Windows ~30Hz 自动重复遇 ≥33ms 帧即同帧合并）按 IME
    /// 本帧实际用掉的次数配给：组合 "n" + 同帧 [退格, 退格] → 第一个被
    /// 用掉（删拼音），第二个直通文本框（删正文）。按值 contains 会把
    /// 同值事件整体误判为已消费，第二个退格静默丢失（#2596-10 变体）。
    /// 纯函数：多个消费方用同一事件批调用得到同一结果，互不消耗配额。
    pub fn unconsumed<'a>(&self, batch: &'a [KeyboardInput]) -> Vec<&'a KeyboardInput> {
        if !self.enabled {
            return batch.iter().collect();
        }
        let mut quota: HashMap<&KeyboardInput, usize> = HashMap::new();
        for k in &self.consumed {
            *quota.entry(k).or_insert(0) += 1;
        }
        batch
            .iter()
            .filter(|k| match quota.get(*k).copied() {
                Some(n) if n > 0 => {
                    quota.insert(*k, n - 1);
                    false
                }
                _ => true,
            })
            .collect()
    }

    /// 本帧是否有被 IME 用掉的 Esc（= 取消组合的那次按键）。
    /// ButtonInput 类消费方（just_pressed 只知帧级真假）据此让路：
    /// 组合中按 Esc 只收候选条，不当帧连带关对话框/关菜单（#2596-7）。
    pub fn escape_consumed(&self) -> bool {
        self.enabled && self.consumed_escape
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
    // 帧首三清（须在任何早退之前生效）：
    // - 上帧消费表（unconsumed 只反映本帧事件）；
    // - 上帧未被取走的提交——提交只在产生帧有效，失焦帧产生的提交若无人取走
    //   会永久滞留，之后注入到任意获焦框（还会封锁该期间的按键判定）
    // - 上帧的 Esc 消费标记（escape_consumed 只反映本帧）
    ime.consumed.clear();
    ime.commit_pending = None;
    ime.consumed_escape = false;

    let key_list: Vec<KeyboardInput> = keys.read().cloned().collect();

    // 1) Shift 单按检测（无论是否聚焦都允许切换中/英）
    for key in &key_list {
        if key.logical_key == Key::Shift {
            if key.state == ButtonState::Pressed && !key.repeat {
                // repeat 的 Pressed 是 OS 自动重复（按住 Shift 打大写期间会持续
                // 产生）——重新武装 clean 会让松开时误判为单按而切换中/英，
                // 丢弃进行中的组合
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
    if focus.rect.is_none() {
        // 失焦即弃组（对齐系统 IME：焦点离开输入框，进行中的组合就地取消）。
        // 旧实现组合跨失焦冻结存活，而 unconsumed 无视焦点 → 组合滞留期间
        // 后续所有字母/数字/退格/Enter 被空组合吞掉——组合拼音后点进密码框，
        // 一个字符都打不进去（focus 为上一帧 Update 的回填，弃组晚一帧生效）
        ime.cancel();
        return;
    }
    for key in &key_list {
        if key.state != ButtonState::Pressed {
            continue;
        }
        // 临时英文段：所有按键直通文本框，不进拼音处理（也不消费）。空格/
        // 回车/Esc 结束段——键本身同样直通（空格进正文、Enter 提交输入框、
        // Esc 归对话框/关输入行），段后自动回中文。
        // 进段见下方字母分支（组合为空的 Shift+大写）；失焦弃组/中英切换出段。
        if ime.temp_english {
            match &key.logical_key {
                Key::Space | Key::Enter | Key::Escape => ime.temp_english = false,
                _ => {}
            }
            continue;
        }
        // 记录「本事件被 IME 消费」：unconsumed 逐事件配给，未消费的事件
        // （如组合为空时的空格/数字/退格）照常到达文本框
        let mut consumed = false;
        match &key.logical_key {
            Key::Character(c) => {
                let s = c.as_str();
                if s.chars().all(|ch| ch.is_ascii_alphabetic()) {
                    if ime.is_composing() || !s.chars().any(|ch| ch.is_ascii_uppercase()) {
                        // 字母进拼音缓冲（小写）
                        for ch in s.chars() {
                            ime.feed_letter(ch.to_ascii_lowercase());
                        }
                        consumed = true;
                    } else {
                        // 组合为空的 Shift+大写字母直通文本框并进入临时英文段
                        // ——中文模式也能整体打出 Bob / A@B.com：只放行单个
                        // 大写的话，'Bob' 后续的 'o' 又会进拼音组合
                        ime.temp_english = true;
                    }
                } else if s.chars().all(|ch| ch.is_ascii_digit()) {
                    // 数字选候选（仅组合中）
                    if ime.is_composing() {
                        if let Ok(n) = s.parse::<usize>() {
                            if n >= 1 {
                                ime.select(n - 1);
                                consumed = true;
                            }
                        }
                    }
                } else if s == "-" {
                    if ime.is_composing() {
                        ime.page_prev();
                        consumed = true;
                    }
                } else if s == "=" {
                    if ime.is_composing() {
                        ime.page_next();
                        consumed = true;
                    }
                }
            }
            Key::Space => {
                if ime.is_composing() {
                    ime.commit_default();
                    consumed = true;
                }
            }
            Key::Enter => {
                if ime.is_composing() {
                    ime.commit_default();
                    consumed = true;
                }
            }
            Key::Backspace => {
                if ime.is_composing() {
                    ime.backspace();
                    consumed = true;
                }
            }
            Key::Escape => {
                if ime.is_composing() {
                    ime.cancel();
                    consumed = true;
                    // 供 ButtonInput 类消费方让路（见 escape_consumed）
                    ime.consumed_escape = true;
                }
            }
            _ => {}
        }
        if consumed {
            ime.consumed.push(key.clone());
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
    // 决定是否显示：中文模式 + 正在组合（聚焦已由上方 let-else 保证）
    let vis = if ime.enabled() && ime.is_composing() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    // 文本：拼音串 + 候选 1.xxx 2.xxx ...（只在聚焦帧构建——失焦帧上面已提前返回）
    let mut label = ime.composing.clone();
    let cands = ime.page_candidates();
    if !cands.is_empty() {
        label.push_str("  ");
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
        app.insert_resource(PinyinIme::new(PinyinDict::load()));
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

    /// Shift 键（可指定 repeat——OS 按住自动重复产生的事件）
    fn shift_key_with(repeat: bool, state: ButtonState) -> KeyboardInput {
        KeyboardInput {
            repeat,
            ..shift_key(state)
        }
    }

    /// 任意逻辑键（按下）——Esc/Enter/Space/Backspace 等非字符键
    fn raw_key(logical: Key) -> KeyboardInput {
        KeyboardInput {
            key_code: KeyCode::Escape, // 逻辑只看 logical_key，物理码随意
            logical_key: logical,
            state: ButtonState::Pressed,
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

    /// unconsumed 契约：只有真正喂进拼音缓冲的那次按键才被配给掉。
    /// 英文模式全批直通；中文模式首字母被用掉（探针批中消失）；未发生的
    /// 键即使组合中也不受影响。
    #[test]
    fn unconsumed_contract() {
        let mut app = ime_app_with_toggled_focus();
        // 英文模式：字母直通
        send(&mut app, char_key("a"));
        app.update();
        assert_eq!(
            app.world()
                .resource::<PinyinIme>()
                .unconsumed(&[char_key("a")])
                .len(),
            1
        );

        // 切中文 + 聚焦，喂首字母 → 该事件被用掉；未发生的 'b' 不受影响
        enable_chinese(&mut app);
        send(&mut app, char_key("a"));
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(ime.is_composing());
        assert!(ime.unconsumed(&[char_key("a")]).is_empty());
        assert_eq!(ime.unconsumed(&[char_key("b")]).len(), 1);
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

    /// 空组合回归：旧实现 segment("") 返回 Some(vec![])，③ 的 `syls.len() - 1`
    /// usize 下溢 panic（登录界面输入字母再 Backspace 删尽可稳定复现）。
    #[test]
    fn lookup_empty_composing_no_panic() {
        let d = PinyinDict::minimal();
        assert!(d.lookup("").is_empty());
        // 根因契约：空串不构成有效切分（Some ⇒ ≥1 个音节），
        // 调用方对结果做 len()-1 类索引才不会再下溢
        assert_eq!(d.segment(""), None);
    }

    /// Backspace 删尽组合后 recompute 不再 panic，状态回到未组合。
    #[test]
    fn backspace_to_empty_no_panic() {
        let mut ime = PinyinIme::new(PinyinDict::minimal());
        ime.toggle(); // 中文模式
        ime.feed_letter('n');
        ime.feed_letter('i');
        assert!(ime.is_composing());
        // "ni" 是有效音节：候选非空——保证后面「删尽后候选复位」断言有区分度
        // （旧用例喂单个 'n'（无效音节），候选删前删后都是空，测不出残留）
        assert!(
            !ime.candidates.is_empty(),
            "前置：minimal() 词典须含音节 ni（候选非空，后续复位断言才有区分度）"
        );
        ime.backspace(); // 删到 "n"（无效音节 → 候选清空，仍在组合）
        assert!(ime.candidates.is_empty()); // 钉死中间态：无效音节的候选确已清空
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

        // 失焦帧：候选条当帧落回 Hidden（PostUpdate 读到 None）；组合在下一帧
        // PreUpdate 弃组（IME 读的是上一帧 Update 的回填，弃组晚一帧生效）
        app.world_mut().resource_mut::<BackfillFocus>().0 = false;
        app.update();
        assert_eq!(*bg.single(app.world()).unwrap(), Visibility::Hidden);
        assert_eq!(*txt.single(app.world()).unwrap(), Visibility::Hidden);
        app.update();
        assert!(!app.world().resource::<PinyinIme>().is_composing());
    }

    // ---- #2596 统一契约（逐事件消费 / 失焦弃组 / 提交过期 / 大写直通 / Shift 重复）----

    /// 组装可切换聚焦的 IME 契约测试 App（弱字体句柄：不 spawn 候选条实体）
    fn ime_app_with_toggled_focus() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(PinyinImePlugin);
        app.add_message::<KeyboardInput>();
        app.insert_resource(crate::ui::sprite_ui::UiFont(Handle::<Font>::default()));
        app.insert_resource(BackfillFocus(true));
        app.add_systems(
            Update,
            |flag: Res<BackfillFocus>, mut f: ResMut<ImeFocus>| {
                if flag.0 {
                    f.rect = Some((10.0, 10.0, 100.0, 16.0));
                }
            },
        );
        app
    }

    /// Shift 单按切到中文模式
    fn enable_chinese(app: &mut App) {
        send(app, shift_key(ButtonState::Pressed));
        app.update();
        send(app, shift_key(ButtonState::Released));
        app.update();
        assert!(app.world().resource::<PinyinIme>().enabled());
    }

    /// #2596-1 密码框死锁回归：组合中途失焦 → 组合就地取消，后续按键不再被吞。
    /// 旧实现组合跨失焦冻结存活，consumes_key 无视焦点吞掉一切字母/数字/退格/
    /// Enter——组合拼音后点进密码框，一个字符都打不进（本测试在旧代码上失败）。
    #[test]
    fn blur_releases_composition_and_keys() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        for c in "ni".chars() {
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        assert!(app.world().resource::<PinyinIme>().is_composing());

        // 失焦 → 组合就地取消（IME 读上一帧的回填，弃组在下一帧 PreUpdate 生效）
        app.world_mut().resource_mut::<BackfillFocus>().0 = false;
        app.update(); // 本帧 PreUpdate 仍见 Some；Update 起不再回填
        app.update(); // 本帧 PreUpdate 见 None → 弃组
        assert!(!app.world().resource::<PinyinIme>().is_composing());

        // 失焦后的字母：不进缓冲、不被配给（可正常到达任意文本框，含密码框）
        send(&mut app, char_key("a"));
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(!ime.is_composing());
        assert_eq!(ime.unconsumed(&[char_key("a")]).len(), 1);
    }

    /// #2596-9 提交帧过期回归：失焦帧产生的提交（无人取走）下一帧即丢弃，
    /// 不滞留注入到之后任意获焦框、不封锁期间的 Enter/空格。旧实现永久滞留。
    #[test]
    fn commit_expires_next_frame() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        for c in "ni".chars() {
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        // 数字 1 选首候选 → commit_pending（本测试无消费者取走）
        send(&mut app, char_key("1"));
        app.update();
        assert!(app.world().resource::<PinyinIme>().has_commit());

        // 下一帧仍未取走 → 过期丢弃（旧实现永久滞留，本断言失败）
        app.update();
        assert!(!app.world().resource::<PinyinIme>().has_commit());
    }

    /// #2596-7 Esc 单键单效回归：取消组合的那次 Esc 被 IME 消费（对话框当帧
    /// 不再同时关闭/清空）；组合已空后的 Esc 不被消费（正常触发对话框动作）。
    #[test]
    fn escape_consumed_exactly_once() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        for c in "ni".chars() {
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        let esc = raw_key(Key::Escape);
        send(&mut app, esc.clone());
        app.update();
        assert!(!app.world().resource::<PinyinIme>().is_composing());
        // 取消组合的 Esc 被消费（旧实现 cancel 后 is_composing=false →
        // 消费表查不到该键，对话框被同一按键连带关闭）
        assert!(app.world().resource::<PinyinIme>().escape_consumed());
        assert!(app
            .world()
            .resource::<PinyinIme>()
            .unconsumed(&[esc])
            .is_empty());

        // 下一帧组合已空 → Esc 不再被消费
        send(&mut app, raw_key(Key::Escape));
        app.update();
        assert!(!app.world().resource::<PinyinIme>().escape_consumed());
        assert_eq!(
            app.world()
                .resource::<PinyinIme>()
                .unconsumed(&[raw_key(Key::Escape)])
                .len(),
            1
        );
    }

    /// #2596-10 同帧批黑洞回归：退格删尽组合后同帧的空格不再被吞。
    /// 旧实现 acting 整体判定（组合中/有提交/刚编辑）把同帧后段未被 IME
    /// 处理的键一并吞掉——帧合并时静默丢失。
    #[test]
    fn same_frame_backspace_drain_space_passes_through() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        for c in "ni".chars() {
            send(&mut app, char_key(&c.to_string()));
            app.update();
        }
        // 同帧批：退格删 'i'、退格删 'n'（删尽）、空格
        let bs = raw_key(Key::Backspace);
        let sp = raw_key(Key::Space);
        send(&mut app, bs.clone());
        send(&mut app, bs);
        send(&mut app, sp.clone());
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(!ime.is_composing());
        // 两次退格都被用掉（删的是拼音）；删尽后的空格直通（可正常上屏）
        assert!(ime.unconsumed(&[raw_key(Key::Backspace)]).is_empty());
        assert_eq!(ime.unconsumed(&[sp]).len(), 1);
    }

    /// #2596-10 同值重复配给回归：组合只够删一次时，同帧第二个退格直通
    /// 文本框（删正文）——按值 contains 会把它也判为已消费而静默丢失
    /// （Windows ~30Hz 自动重复在 ≥33ms 帧内合并的实况，#2596-10 记录）。
    #[test]
    fn same_frame_duplicate_backspace_rations_by_consumed_count() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        send(&mut app, char_key("n")); // 'n' 单字母：组合长度 1，只够删一次
        app.update();
        assert!(app.world().resource::<PinyinIme>().is_composing());

        let bs1 = raw_key(Key::Backspace);
        let bs2 = raw_key(Key::Backspace); // 与 bs1 完全等值（真实合并即如此）
        send(&mut app, bs1.clone());
        send(&mut app, bs2);
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(!ime.is_composing(), "第一个退格删尽拼音组合");
        // 配给：批中 2 个退格只配掉 1 个——第二个必须直通（删正文一字符）
        let batch = [bs1.clone(), bs1];
        let left = ime.unconsumed(&batch);
        assert_eq!(left.len(), 1, "同值重复按用掉次数配给，第二个退格直通");
    }

    /// #2596-12 临时英文段回归：中文模式组合为空时 Shift+大写字母直通并进段，
    /// 段内后续小写继续直通（'Bob' 整体可打），空格结束段后自动回中文。
    /// 旧实现一律 to_ascii_lowercase 喂入 → 'Bob' 变 'bob'；只放行单个大写
    /// 的话 'o' 又进拼音组合。
    #[test]
    fn uppercase_enters_temp_english_segment() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        send(&mut app, char_key("B"));
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(!ime.is_composing(), "组合为空时大写字母应直通，不进缓冲");
        assert!(ime.temp_english, "Shift+大写进入临时英文段");

        // 段内小写继续直通（'Bob' 的 'o' 不进拼音组合）
        send(&mut app, char_key("o"));
        app.update();
        let ime = app.world().resource::<PinyinIme>();
        assert!(ime.temp_english);
        assert!(!ime.is_composing(), "临时英文段内字母不进拼音缓冲");

        // 空格结束段（本身直通）→ 后续字母回中文拼音
        send(&mut app, raw_key(Key::Space));
        app.update();
        assert!(!app.world().resource::<PinyinIme>().temp_english);
        send(&mut app, char_key("n"));
        app.update();
        assert!(app.world().resource::<PinyinIme>().is_composing());
    }

    /// #2596-12 实况序列：'A@B.com' 整体打出——'A' 进段，'@'/'B'/'.'/'com'
    /// 全部直通（无组合、无候选条），段被空格/回车/Esc/失焦结束。
    #[test]
    fn temp_english_types_email_address() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        for ch in ["A", "@", "B", ".", "c", "o", "m"] {
            send(&mut app, char_key(ch));
            app.update();
        }
        let ime = app.world().resource::<PinyinIme>();
        assert!(ime.temp_english, "段持续到显式结束键");
        assert!(!ime.is_composing(), "段内符号/字母都不进拼音");

        // Esc 结束段且直通（归对话框/输入行，不进拼音）
        send(&mut app, raw_key(Key::Escape));
        app.update();
        assert!(!app.world().resource::<PinyinIme>().temp_english);
    }

    /// #2596-11 Shift 自动重复回归：按住 Shift（期间按过其它键，作修饰键用）
    /// 时 OS 重复派发的 Shift Pressed 不再重新武装单按判定——松开不误切中/英。
    #[test]
    fn shift_repeat_does_not_rearm_clean() {
        let mut app = ime_app_with_toggled_focus();
        enable_chinese(&mut app);
        // 按下 Shift（clean）→ 按住期间按 'B'（clean=false，修饰用途）
        send(&mut app, shift_key(ButtonState::Pressed));
        app.update();
        send(&mut app, char_key("B"));
        app.update();
        // OS 自动重复的 Shift Pressed（repeat=true）→ 旧实现把 clean 重新置 true
        send(&mut app, shift_key_with(true, ButtonState::Pressed));
        app.update();
        send(&mut app, shift_key(ButtonState::Released));
        app.update();
        assert!(
            app.world().resource::<PinyinIme>().enabled(),
            "按住 Shift 用作修饰键 + 自动重复 → 松开不应切换中/英"
        );
    }
}
