//! 跨端共享的**输入校验规则**（客户端与服务端必须同一份真源）。
//!
//! 为什么放共享 crate：角色名规则一度在两端各写一份 —— 服务端 `3..=15`（对齐 C#
//! `Envir.CharacterReg` = `[\u4e00-\u9fa5_A-Za-z0-9]{3,15}`），客户端却是 `1..=15`。
//! 于是"2 字中文名"在客户端**显示合法**（确定键可用、名字边框不报警），提交后被服务端静默拒绝
//! （`NewCharacter rejected: invalid name`，且这条分支**不回包**）——玩家看到的就是
//! "点了确定没反应 / 无法创建角色"（owner 2026-09-25 反馈）。
//!
//! 这类"规则各写一份"的漂移不会报错，只会让**合法/非法在两端不一致**；规则只能有一份。

/// 角色名长度下限（按**字符**计，不是字节；中文按 1 个字符算）
pub const MIN_CHARACTER_NAME_CHARS: usize = 3;

/// 角色名长度上限（按字符计）
pub const MAX_CHARACTER_NAME_CHARS: usize = 15;

/// 角色名是否合法：`MIN..=MAX` 个字符，字符集 = 中文（U+4E00..=U+9FA5）/ 下划线 / ASCII 字母数字。
///
/// 客户端用它决定"确定键可用 + 名字边框颜色"，服务端用它做最终校验 —— 两端同源，
/// 不会再出现"客户端说合法、服务端说非法"的静默失败。
pub fn character_name_valid(name: &str) -> bool {
    let chars = name.chars().count();
    if !(MIN_CHARACTER_NAME_CHARS..=MAX_CHARACTER_NAME_CHARS).contains(&chars) {
        return false;
    }
    name.chars()
        .all(|c| c == '_' || c.is_ascii_alphanumeric() || ('\u{4E00}'..='\u{9FA5}').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_too_short_names() {
        // 这两条正是 owner 报的"无法创建角色"最可能的输入形态（2 字中文名）
        assert!(!character_name_valid(""));
        assert!(!character_name_valid("a"));
        assert!(!character_name_valid("ab"));
        assert!(
            !character_name_valid("小明"),
            "2 字中文名必须判非法（服务端就是这么判的）"
        );
    }

    #[test]
    fn accepts_boundary_lengths() {
        assert!(character_name_valid("abc"));
        assert!(character_name_valid("小明明"));
        assert!(character_name_valid("abc_def"));
        assert!(character_name_valid(&"x".repeat(MAX_CHARACTER_NAME_CHARS)));
        assert!(!character_name_valid(
            &"x".repeat(MAX_CHARACTER_NAME_CHARS + 1)
        ));
    }

    #[test]
    fn rejects_other_characters() {
        // 标点/空格/其它脚本（韩文/emoji）都不在 C# Envir.CharacterReg 里
        assert!(!character_name_valid("abc def"));
        assert!(!character_name_valid("abc-def"));
        assert!(!character_name_valid("小明！好"));
        assert!(!character_name_valid("한국어"));
        assert!(!character_name_valid("😀😀😀"));
    }
}
