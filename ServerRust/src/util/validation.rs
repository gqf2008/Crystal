//! Phase 1.3: 输入验证 — 防止恶意/超大输入到达业务逻辑。
//!
//! 所有函数返回 `bool`(true = 合法,false = 拒绝)。
//! 调用方在拒绝时 log warn 并跳过处理。

/// 最大聊天消息长度(字节)。master C# 默认 512。
pub const MAX_CHAT_LEN: usize = 512;

/// 最大用户名长度。
pub const MAX_USERNAME_LEN: usize = 20;

/// 最小用户名长度。
pub const MIN_USERNAME_LEN: usize = 3;

/// 最大密码长度。
pub const MAX_PASSWORD_LEN: usize = 32;

/// 最大角色名长度。
pub const MAX_CHAR_NAME_LEN: usize = 12;

/// 最小角色名长度。
pub const MIN_CHAR_NAME_LEN: usize = 2;

/// 最大 NPC 输入长度(对话框文本输入)。
pub const MAX_NPC_INPUT_LEN: usize = 64;

/// 最大通用字符串长度(防御性默认值)。
pub const MAX_GENERIC_STRING_LEN: usize = 1024;

/// 验证聊天消息:长度限制 + 非 control characters 泛滥。
pub fn validate_chat(msg: &str) -> bool {
    if msg.is_empty() || msg.len() > MAX_CHAT_LEN {
        return false;
    }
    // 拒绝连续 null bytes 或纯 control characters
    let control_count = msg.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\t').count();
    control_count < msg.chars().count() / 2
}

/// 验证用户名:3-20 字符,字母数字 + 下划线。
pub fn validate_username(name: &str) -> bool {
    if name.len() < MIN_USERNAME_LEN || name.len() > MAX_USERNAME_LEN {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 验证密码:非空,≤32 字符。
pub fn validate_password(pw: &str) -> bool {
    !pw.is_empty() && pw.len() <= MAX_PASSWORD_LEN
}

/// 验证角色名:2-12 字符,允许中文 + 字母数字。
pub fn validate_character_name(name: &str) -> bool {
    if name.len() < MIN_CHAR_NAME_LEN || name.len() > MAX_CHAR_NAME_LEN {
        return false;
    }
    // 允许:字母、数字、中文(CJK Unified Ideographs)、部分标点
    name.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || ('\u{4E00}'..='\u{9FFF}').contains(&c)  // CJK
            || c == '_'
    })
}

/// 验证 NPC 对话输入:≤64 字符,无 control chars。
pub fn validate_npc_input(input: &str) -> bool {
    if input.len() > MAX_NPC_INPUT_LEN {
        return false;
    }
    !input.chars().any(|c| c.is_control())
}

/// 通用字符串长度限制(防御性:防止超大 payload 导致 OOM)。
pub fn validate_generic_string(s: &str) -> bool {
    s.len() <= MAX_GENERIC_STRING_LEN
}
