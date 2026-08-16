// ============================================================================
// text_markup - 对话文本标记渲染的共享度量/色彩工具
// 从 npc.rs 抽出（#2602 批R：公告对话框复用同一叠加标签架构）
// 消费方：game/dialogs/npc.rs（NPC 对话）、game/dialogs/notice.rs（公告）
// ============================================================================

use bevy::prelude::*;

/// 文本估宽（逻辑 px）：宋体是双宽度量字体——CJK/全角 advance 恒为 1.00em
/// （=字号，估即精确），ASCII（含空格）恒为 0.50em（实测 upem 256/advance 128）。
/// 基础白字与叠加标签同用宋体排版，估宽与实际度量一致，叠加段定位无累积漂移；
/// 对应 C# MeasureText(prefix)-10 的定位职责
pub fn est_text_width(s: &str, size: f32) -> f32 {
    s.chars()
        .map(|c| {
            if (c as u32) >= 0x2E80 {
                size
            } else {
                size * 0.5
            }
        })
        .sum()
}

/// 文本按最大宽度折行（公告 WordBreak 用）：逐字符贪心累计，超宽即断行。
/// CJK 任意处可断；拉丁尽量在空格处断（回溯到最后一个空格）
pub fn wrap_text(s: &str, size: f32, max_w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    let mut line_w = 0.0f32;
    let mut last_space: Option<usize> = None; // line 内字节偏移
    for ch in s.chars() {
        let cw = est_text_width(&ch.to_string(), size);
        if line_w + cw > max_w && !line.is_empty() {
            // 拉丁词中断：回溯到最后一个空格（其后的字符挪到下一行）
            if ch != ' ' && (ch as u32) < 0x2E80 {
                if let Some(sp) = last_space {
                    let head = line[..sp].to_string();
                    let tail = line[sp + 1..].to_string();
                    let tail_w = est_text_width(&tail, size);
                    out.push(head);
                    line = tail;
                    line_w = tail_w;
                    last_space = None;
                    line.push(ch);
                    line_w += cw;
                    continue;
                }
            }
            out.push(std::mem::take(&mut line));
            line_w = 0.0;
            last_space = None;
            if ch == ' ' {
                continue; // 行首不保留折行产生的空格
            }
        }
        if ch == ' ' {
            last_space = Some(line.len());
        }
        line.push(ch);
        line_w += cw;
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// C# Color.FromName KnownColor 真值 → srgb（脚本实际使用的子集 + 常见色）
pub fn known_color(name: &str) -> Option<Color> {
    let rgb = match name.to_ascii_lowercase().as_str() {
        "red" => (1.0, 0.0, 0.0),
        "darkred" => (0.55, 0.0, 0.0),
        "crimson" => (0.86, 0.08, 0.24),
        "indianred" => (0.8, 0.36, 0.36),
        "tomato" => (1.0, 0.39, 0.28),
        "orangered" => (1.0, 0.27, 0.0),
        "orange" => (1.0, 0.65, 0.0),
        "coral" => (1.0, 0.5, 0.31),
        "gold" => (1.0, 0.84, 0.0),
        "goldenrod" => (0.86, 0.65, 0.13),
        "greenyellow" => (0.68, 1.0, 0.18),
        "yellow" => (1.0, 1.0, 0.0),
        "khaki" => (0.94, 0.9, 0.55),
        "wheat" => (0.96, 0.87, 0.7),
        "green" => (0.0, 0.5, 0.0),
        "darkgreen" => (0.0, 0.39, 0.0),
        "seagreen" => (0.18, 0.55, 0.34),
        "forestgreen" => (0.13, 0.55, 0.13),
        "limegreen" => (0.2, 0.8, 0.2),
        "springgreen" => (0.0, 1.0, 0.5),
        "cyan" | "aqua" => (0.0, 1.0, 1.0),
        "teal" => (0.0, 0.5, 0.5),
        "blue" => (0.0, 0.0, 1.0),
        "darkblue" => (0.0, 0.0, 0.55),
        "dodgerblue" => (0.12, 0.56, 1.0),
        "skyblue" => (0.53, 0.81, 0.92),
        "deepskyblue" => (0.0, 0.75, 1.0),
        "royalblue" => (0.25, 0.41, 0.88),
        "lightsteelblue" => (0.69, 0.77, 0.87),
        "steelblue" => (0.27, 0.51, 0.71),
        "purple" => (0.5, 0.0, 0.5),
        "violet" => (0.93, 0.51, 0.93),
        "magenta" | "fuchsia" => (1.0, 0.0, 1.0),
        "plum" => (0.87, 0.63, 0.87),
        "pink" => (1.0, 0.75, 0.8),
        "hotpink" => (1.0, 0.41, 0.71),
        "brown" => (0.65, 0.16, 0.16),
        "chocolate" => (0.82, 0.41, 0.12),
        "gray" | "grey" => (0.5, 0.5, 0.5),
        "darkgray" | "darkgrey" => (0.66, 0.66, 0.66),
        "silver" => (0.75, 0.75, 0.75),
        "lightgray" | "lightgrey" => (0.83, 0.83, 0.83),
        "black" => (0.0, 0.0, 0.0),
        "white" => (1.0, 1.0, 1.0),
        _ => return None,
    };
    Some(Color::srgb(rgb.0, rgb.1, rgb.2))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 宋体双宽度量：ASCII 恒 0.50em、CJK/全角恒 1.00em（upem 256 实测）——
    /// 叠加段定位/链接命中区间与实际排版 advance 一致，不得漂移
    #[test]
    fn est_width_dual_metrics() {
        assert_eq!(est_text_width("ab", 13.0), 13.0);
        assert_eq!(est_text_width("古", 13.0), 13.0);
        assert_eq!(est_text_width("a古b", 13.0), 26.0);
    }

    /// KHAKI 等 {t/Color} 高频色与 KnownColor 真值一致（e2e 断言依赖）
    #[test]
    fn known_colors_match_knowncolor() {
        let khaki = known_color("KHAKI").unwrap();
        assert_eq!(khaki, Color::srgb(0.94, 0.9, 0.55));
        assert_eq!(known_color("Orange").unwrap(), Color::srgb(1.0, 0.65, 0.0));
        assert_eq!(
            known_color("LightSteelBlue").unwrap(),
            Color::srgb(0.69, 0.77, 0.87)
        );
        assert!(known_color("NotAColor").is_none());
    }

    /// 折行：CJK 满 42 字/行（10px×420）、拉丁在空格处断、短行原样
    #[test]
    fn wrap_basics() {
        // 50 个 CJK @10px = 500px > 420 → 两行（42+8）
        let long = "字".repeat(50);
        let wrapped = wrap_text(&long, 10.0, 420.0);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0].chars().count(), 42);
        assert_eq!(wrapped[1].chars().count(), 8);

        // 拉丁按空格断行，行首不留空格
        let words = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll";
        let wrapped = wrap_text(words, 10.0, 105.0);
        assert!(wrapped.iter().all(|l| !l.starts_with(' ')));
        assert!(wrapped.iter().all(|l| est_text_width(l, 10.0) <= 105.0));
        // 无字丢失
        let joined: String = wrapped.join(" ");
        assert_eq!(joined.replace("  ", " "), words);

        // 短行/空串
        assert_eq!(wrap_text("hi", 10.0, 420.0), vec!["hi".to_string()]);
        assert_eq!(wrap_text("", 10.0, 420.0).len(), 1);
    }
}
