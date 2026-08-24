// ============================================================================
// libpinyin 内置拼音输入法引擎（与 mir2x 一致）
// ============================================================================
// 直接封装 GNU libpinyin（mir2x 通过 vcpkg 引入的 etorth/libpinyin fork + model20 数据），
// 提供整句拼音预测（pinyin_guess_sentence_with_prefix + 候选堆栈逐段累积成句）、
// 不完整拼音(PINYIN_INCOMPLETE)、模糊纠正(PINYIN_CORRECT_ALL)、divided/resplit 切分、
// 动态词频(DYNAMIC_ADJUST)。
//
// 交互模型（按独立审查 B2 修正）：**选中即上屏 + 剩余拼音重新组合**——
// 选词时立刻把选中词提交给输入框，并把未消费的剩余拼音作为新组合继续（每次 recompute
// 都对当前 input 重新 pinyin_reset + parse，保证新字母一定被解析，避免 stk 状态下
// “选词后续打”候选停滞/上屏错乱）。
//
// 候选窗口保留在 pinyin_ime.rs 的 PinyinBar（自绘候选条）。
// ----------------------------------------------------------------------------

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::CStr;

// ---- FFI 类型 ----
type pinyin_context_t = std::ffi::c_void;
type pinyin_instance_t = std::ffi::c_void;
type lookup_candidate_t = std::ffi::c_void;
type guint = std::ffi::c_uint;
type gchar = std::ffi::c_char;

// ---- libpinyin 选项位（etorth/libpinyin fork: src/storage/pinyin_custom2.h）----
const PINYIN_INCOMPLETE: guint = 0x8; // 1U<<3
const USE_DIVIDED_TABLE: guint = 0x80; // 1U<<7
const USE_RESPLIT_TABLE: guint = 0x100; // 1U<<8
const DYNAMIC_ADJUST: guint = 0x200; // 1U<<9
const PINYIN_CORRECT_ALL: guint = 0xFF << 21; // 0x1FE00000
const SORT_BY_PHRASE_LENGTH_AND_PINYIN_LENGTH_AND_FREQUENCY: guint = 0x1E; // fork pinyin.h sort_option_t

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

/// libpinyin 引擎封装。每个实例持有一个 context + instance（仅主线程访问，故只需 Send）。
pub struct LibpinyinEngine {
    context: *mut pinyin_context_t,
    instance: *mut pinyin_instance_t,
    /// 用户已输入的原始拼音串（选中部分提交后，这里只剩剩余拼音）
    input: String,
    /// 句子预测用的前缀（当前为空串；保留接口对齐 mir2x）
    prefix: String,
    /// 当前候选列表
    candidates: Vec<String>,
}

// Bevy Resource 要求 Send + Sync。libpinyin 的 context/instance 非线程安全，这里手动标记。
// SAFETY: context/instance 仅由 pinyin_ime_system（PreUpdate，单系统独占 ResMut）经 &mut self
// 调用 libpinyin FFI；UI 系统只经 &self 读已缓存的自绘候选（candidates Vec），不触碰 FFI。
// 同一时刻至多一个系统访问该资源，且不跨线程共享，故 `&LibpinyinEngine` 跨线程共享与
// 并发调用 FFI 均不会发生 —— Send + Sync 均安全。
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
                candidates: Vec::new(),
            })
        }
    }

    /// 清空输入与候选（放弃当前组合）。
    pub fn clear(&mut self) {
        unsafe {
            pinyin_reset(self.instance);
        }
        self.input.clear();
        self.prefix.clear();
        self.candidates.clear();
    }

    pub fn feed(&mut self, c: char) {
        self.input.push(c);
        self.recompute();
    }

    pub fn backspace(&mut self) {
        self.input.pop();
        self.recompute();
    }

    /// 重置并设置输入串（选中词提交后，把剩余拼音喂回来）。
    pub fn set_input(&mut self, input: String) {
        self.input = input;
        self.recompute();
    }

    pub fn assign(&mut self, prefix: String, input: String) {
        self.prefix = prefix;
        self.input = input;
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

    /// 选中第 idx 个候选：返回 (选中词, 已消费的拼音字节数)，并重置实例。
    /// 调用方负责把剩余拼音经 set_input 喂回继续组合。
    pub fn choose(&mut self, idx: usize) -> Option<(String, usize)> {
        if idx >= self.candidates.len() {
            return None;
        }
        unsafe {
            let mut cand: *mut lookup_candidate_t = std::ptr::null_mut();
            if !pinyin_get_candidate(self.instance, idx as guint, &mut cand) || cand.is_null() {
                return None;
            }
            let mut word_ptr: *const gchar = std::ptr::null();
            if !pinyin_get_candidate_string(self.instance, cand, &mut word_ptr) || word_ptr.is_null() {
                return None;
            }
            let word = CStr::from_ptr(word_ptr).to_string_lossy().into_owned();
            // 偏移 0：本实现每次都在“当前输入从头开始”的组合上选词
            let new_offset = pinyin_choose_candidate(self.instance, 0, cand).max(0) as usize;
            pinyin_reset(self.instance);
            Some((word, new_offset))
        }
    }

    /// 重建候选：对当前 input 重新 parse + guess（保证增量输入一定被解析）。
    fn recompute(&mut self) {
        unsafe {
            if self.input.is_empty() {
                self.candidates.clear();
                pinyin_reset(self.instance);
                return;
            }
            let c_input = std::ffi::CString::new(self.input.as_str()).unwrap_or_default();
            let c_prefix = std::ffi::CString::new(self.prefix.as_str()).unwrap_or_default();
            pinyin_reset(self.instance);
            pinyin_parse_more_full_pinyins(self.instance, c_input.as_ptr());
            pinyin_guess_sentence_with_prefix(self.instance, c_prefix.as_ptr());
            self.candidates.clear();
            if pinyin_guess_candidates(self.instance, 0, SORT_BY_PHRASE_LENGTH_AND_PINYIN_LENGTH_AND_FREQUENCY) {
                let mut num: guint = 0;
                if pinyin_get_n_candidate(self.instance, &mut num) {
                    for i in 0..num {
                        let mut cand: *mut lookup_candidate_t = std::ptr::null_mut();
                        if !pinyin_get_candidate(self.instance, i, &mut cand) || cand.is_null() {
                            continue;
                        }
                        let mut word_ptr: *const gchar = std::ptr::null();
                        if !pinyin_get_candidate_string(self.instance, cand, &mut word_ptr) || word_ptr.is_null() {
                            continue;
                        }
                        self.candidates.push(CStr::from_ptr(word_ptr).to_string_lossy().into_owned());
                    }
                }
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
