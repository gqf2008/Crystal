// ============================================================================
// libpinyin 内置拼音输入法引擎（与 mir2x 一致）
// ============================================================================
// 直接封装 GNU libpinyin（mir2x 通过 vcpkg 引入的 etorth/libpinyin fork + model20 数据），
// 提供整句拼音预测（pinyin_guess_sentence_with_prefix + 候选堆栈 stk 逐段累积成句）、
// 不完整拼音(PINYIN_INCOMPLETE)、模糊纠正(PINYIN_CORRECT_ALL)、divided/resplit 切分、
// 动态词频(DYNAMIC_ADJUST)。
//
// 与原 mir2x ime.cpp 的差异：本模块用同步 recompute（游戏 IME 仅在 PreUpdate 单线程访问），
// 不启后台线程；候选窗口保留在 pinyin_ime.rs 的 PinyinBar（自绘候选条）。
// ----------------------------------------------------------------------------

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::CStr;

// ---- FFI 类型 ----
type pinyin_context_t = std::ffi::c_void;
type pinyin_instance_t = std::ffi::c_void;
type lookup_candidate_t = std::ffi::c_void;
type lookup_candidate_type_t = std::ffi::c_int;
type guint = std::ffi::c_uint;
type gchar = std::ffi::c_char;

// ---- libpinyin 选项（pinyin.h enum）----
const PINYIN_INCOMPLETE: guint = 0x1;
const PINYIN_CORRECT_ALL: guint = 0x4;
const USE_DIVIDED_TABLE: guint = 0x8;
const USE_RESPLIT_TABLE: guint = 0x10;
const DYNAMIC_ADJUST: guint = 0x20;
const SORT_BY_PHRASE_LENGTH_AND_PINYIN_LENGTH_AND_FREQUENCY: guint = 0x1E;
// lookup_candidate_type_t
const NBEST_MATCH_CANDIDATE: lookup_candidate_type_t = 0;
const NORMAL_MATCH_CANDIDATE: lookup_candidate_type_t = 1;

extern "C" {
    fn pinyin_init(systemdir: *const gchar, userdir: *const gchar) -> *mut pinyin_context_t;
    fn pinyin_fini(context: *mut pinyin_context_t);
    fn pinyin_set_options(context: *mut pinyin_context_t, options: guint);
    fn pinyin_alloc_instance(context: *mut pinyin_context_t) -> *mut pinyin_instance_t;
    fn pinyin_free_instance(instance: *mut pinyin_instance_t);
    fn pinyin_parse_more_full_pinyins(instance: *mut pinyin_instance_t, pinyins: *const gchar) -> usize;
    fn pinyin_guess_sentence_with_prefix(instance: *mut pinyin_instance_t, prefix: *const gchar) -> bool;
    fn pinyin_guess_candidates(instance: *mut pinyin_instance_t, offset: usize, sort_option: guint) -> bool;
    fn pinyin_get_n_candidate(instance: *mut pinyin_instance_t, num: *mut guint) -> bool;
    fn pinyin_get_candidate(
        instance: *mut pinyin_instance_t,
        index: guint,
        candidate: *mut *mut lookup_candidate_t,
    ) -> bool;
    fn pinyin_get_candidate_type(
        instance: *mut pinyin_instance_t,
        candidate: *mut lookup_candidate_t,
        type_: *mut lookup_candidate_type_t,
    ) -> bool;
    fn pinyin_get_candidate_string(
        instance: *mut pinyin_instance_t,
        candidate: *mut lookup_candidate_t,
        utf8_str: *mut *const gchar,
    ) -> bool;
    fn pinyin_choose_candidate(
        instance: *mut pinyin_instance_t,
        offset: usize,
        candidate: *mut lookup_candidate_t,
    ) -> std::ffi::c_int;
    fn pinyin_reset(instance: *mut pinyin_instance_t) -> bool;
    fn pinyin_save(context: *mut pinyin_context_t) -> bool;
    fn pinyin_mask_out(context: *mut pinyin_context_t, unknown_1: guint, unknown_2: guint) -> bool;
}

/// libpinyin 引擎封装。每个实例持有一个 context + instance（仅在本线程访问，故不需要 Sync）。
pub struct LibpinyinEngine {
    context: *mut pinyin_context_t,
    instance: *mut pinyin_instance_t,
    /// 用户已输入的原始拼音串
    input: String,
    /// 句子预测用的前缀（当前为空串；保留接口对齐 mir2x）
    prefix: String,
    /// 待选中的候选下标（选中后在下一次 recompute 处理）
    selection: Option<usize>,
    /// 当前候选列表
    candidates: Vec<String>,
    /// 已选候选堆栈：(累计句子, 当前拼音偏移)
    stk: Vec<(String, usize)>,
}

// raw pointer 仅在本机单线程访问（游戏 PreUpdate 一个系统），手动标记 Send 以便放进 Bevy Resource。
unsafe impl Send for LibpinyinEngine {}
unsafe impl Sync for LibpinyinEngine {}

impl LibpinyinEngine {
    /// 用 libpinyin 系统/用户数据目录创建。数据目录含 pinyin_index.bin、bigram.db 等。
    pub fn new(data_dir: &str, conf_dir: &str) -> Option<Self> {
        // conf 目录可能不存在（mir2x 运行时创建 user.conf），先建好；
        // 空 user.conf 可压制 libpinyin 的 "open user.conf failed" 日志（对齐 mir2x ime.cpp 构造器）
        let _ = std::fs::create_dir_all(conf_dir);
        let user_conf = std::path::Path::new(conf_dir).join("user.conf");
        if !user_conf.exists() {
            let _ = std::fs::File::create(&user_conf);
        }
        let c_data = std::ffi::CString::new(data_dir).ok()?;
        let c_conf = std::ffi::CString::new(conf_dir).ok()?;
        unsafe {
            let context = pinyin_init(c_data.as_ptr(), c_conf.as_ptr());
            if context.is_null() {
                return None;
            }
            pinyin_set_options(
                context,
                PINYIN_INCOMPLETE | PINYIN_CORRECT_ALL | USE_DIVIDED_TABLE | USE_RESPLIT_TABLE | DYNAMIC_ADJUST,
            );
            let instance = pinyin_alloc_instance(context);
            if instance.is_null() {
                pinyin_fini(context);
                return None;
            }
            Some(Self {
                context,
                instance,
                input: String::new(),
                prefix: String::new(),
                selection: None,
                candidates: Vec::new(),
                stk: Vec::new(),
            })
        }
    }

    /// 清空输入、候选、堆栈（放弃当前组合）。
    pub fn clear(&mut self) {
        unsafe {
            pinyin_reset(self.instance);
        }
        self.input.clear();
        self.prefix.clear();
        self.selection = None;
        self.candidates.clear();
        self.stk.clear();
    }

    pub fn feed(&mut self, c: char) {
        self.input.push(c);
        self.recompute();
    }

    pub fn backspace(&mut self) {
        if self.stk.is_empty() {
            self.input.pop();
        } else {
            self.stk.pop();
        }
        self.recompute();
    }

    pub fn assign(&mut self, prefix: String, input: String) {
        self.stk.clear();
        self.selection = None;
        self.prefix = prefix;
        self.input = input;
        self.recompute();
    }

    /// 选中第 idx 个候选（下次 recompute 处理）。
    pub fn select(&mut self, idx: usize) {
        self.selection = Some(idx);
        self.recompute();
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    /// 当前候选列表（供自绘候选条）。
    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }

    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }

    /// 整个输入串是否已被候选堆栈消费完（可整句上屏）。
    pub fn done(&self) -> bool {
        !self.stk.is_empty() && self.stk.last().map(|(_, off)| *off >= self.input.len()).unwrap_or(false)
    }

    /// 上屏结果：已选句 + 剩余未消费拼音。
    pub fn result(&self) -> String {
        if let Some((sentence, off)) = self.stk.last() {
            let mut s = sentence.clone();
            if *off < self.input.len() {
                s.push_str(&self.input[*off..]);
            }
            s
        } else {
            self.input.clone()
        }
    }

    /// 已累积成句的部分（done() 后等于上屏结果）。
    pub fn sentence(&self) -> String {
        self.stk.last().map(|(s, _)| s.clone()).unwrap_or_default()
    }

    /// 重建候选：处理 pending selection → 维护堆栈 → guess candidates。
    fn recompute(&mut self) {
        let ins = self.instance;
        unsafe {
            if self.input.is_empty() {
                self.prefix.clear();
                self.selection = None;
                self.candidates.clear();
                self.stk.clear();
                pinyin_reset(ins);
                return;
            }

            // 1) 处理选中
            if let Some(sel) = self.selection {
                let mut num: guint = 0;
                pinyin_get_n_candidate(ins, &mut num);
                if sel < num as usize {
                    let choice = sel;
                    self.selection = None;
                    let mut cand: *mut lookup_candidate_t = std::ptr::null_mut();
                    pinyin_get_candidate(ins, choice as guint, &mut cand);
                    let mut word_ptr: *const gchar = std::ptr::null();
                    pinyin_get_candidate_string(ins, cand, &mut word_ptr);
                    let word = CStr::from_ptr(word_ptr).to_string_lossy().into_owned();
                    let mut typ: lookup_candidate_type_t = 0;
                    pinyin_get_candidate_type(ins, cand, &mut typ);
                    let (sentence, offset) = if (typ == NBEST_MATCH_CANDIDATE) || self.stk.is_empty() {
                        (word.clone(), 0)
                    } else {
                        let (prev_sentence, prev_off) = self.stk.last().unwrap().clone();
                        (prev_sentence + &word, prev_off)
                    };
                    let chosen = pinyin_choose_candidate(ins, offset, cand);
                    self.stk.push((sentence, chosen.max(0) as usize));
                } else {
                    self.selection = None;
                }
            } else if self.stk.is_empty() {
                let c_input = std::ffi::CString::new(self.input.as_str()).unwrap_or_default();
                let c_prefix = std::ffi::CString::new(self.prefix.as_str()).unwrap_or_default();
                pinyin_parse_more_full_pinyins(ins, c_input.as_ptr());
                pinyin_guess_sentence_with_prefix(ins, c_prefix.as_ptr());
            }

            // 2) 候选
            self.candidates.clear();
            let offset = if self.stk.is_empty() { 0 } else { self.stk.last().unwrap().1 };
            pinyin_guess_candidates(ins, offset, SORT_BY_PHRASE_LENGTH_AND_PINYIN_LENGTH_AND_FREQUENCY);
            let mut num: guint = 0;
            pinyin_get_n_candidate(ins, &mut num);
            for i in 0..num {
                let mut cand: *mut lookup_candidate_t = std::ptr::null_mut();
                pinyin_get_candidate(ins, i, &mut cand);
                let mut word_ptr: *const gchar = std::ptr::null();
                pinyin_get_candidate_string(ins, cand, &mut word_ptr);
                self.candidates.push(CStr::from_ptr(word_ptr).to_string_lossy().into_owned());
            }
        }
    }
}

impl Drop for LibpinyinEngine {
    fn drop(&mut self) {
        unsafe {
            pinyin_free_instance(self.instance);
            pinyin_mask_out(self.context, 0, 0);
            let _ = pinyin_save(self.context);
            pinyin_fini(self.context);
        }
    }
}
