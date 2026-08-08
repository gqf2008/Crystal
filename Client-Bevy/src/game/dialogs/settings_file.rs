//! 客户端设置文件共用读写（C# Settings.cs：`.\Mir2Config.ini`）
//! 多个对话框（Option/Chat）共享同一文件，各自按 section 合并写回，避免互相覆盖。

use std::fs;

/// 设置文件路径（与 C# Settings.cs InIReader 同路径）
pub const SETTINGS_PATH: &str = "./Mir2Config.ini";

/// 读取设置文件全文（不存在返回空串）
pub fn load_ini() -> String {
    fs::read_to_string(SETTINGS_PATH).unwrap_or_default()
}

/// 写回设置文件全文
pub fn write_ini(content: &str) {
    let _ = fs::write(SETTINGS_PATH, content);
}

/// 读取某 section 某 key 的原始值（section/key 大小写不敏感）
pub fn ini_str<'a>(content: &'a str, section: &str, key: &str) -> Option<&'a str> {
    let mut cur = "";
    for line in content.lines() {
        let l = line.trim();
        if l.starts_with('[') && l.ends_with(']') {
            cur = &l[1..l.len() - 1];
            continue;
        }
        if cur.eq_ignore_ascii_case(section) {
            if let Some(eq) = l.find('=') {
                if l[..eq].trim().eq_ignore_ascii_case(key) {
                    return Some(l[eq + 1..].trim());
                }
            }
        }
    }
    None
}

/// 读取 INI 布尔值（缺省/非法回退 default）
pub fn ini_bool(content: &str, section: &str, key: &str, default: bool) -> bool {
    ini_str(content, section, key)
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(default)
}

/// 读取 INI 百分比音量（0-100 → 0.0-1.0）
pub fn ini_percent(content: &str, section: &str, key: &str, default: f32) -> f32 {
    ini_str(content, section, key)
        .and_then(|v| v.parse::<f32>().ok())
        .map(|v| (v / 100.0).clamp(0.0, 1.0))
        .unwrap_or(default)
}

/// 在指定 section upsert `key=value`，保留文件其余内容（C# InIReader.Write 语义）
pub fn set_ini_value(content: &str, section: &str, key: &str, value: &str) -> String {
    let section_l = section.to_lowercase();
    let key_l = key.to_lowercase();
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut key_written = false;
    let mut section_seen = false;

    for raw in content.lines() {
        let line = raw.trim_end().to_string();
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec = trimmed[1..trimmed.len() - 1].to_lowercase();
            if cur == section_l && !key_written {
                out.push(format!("{}={}", key, value));
                key_written = true;
            }
            if sec == section_l {
                section_seen = true;
            }
            cur = sec;
            out.push(line);
            continue;
        }
        if cur == section_l {
            if let Some(eq) = line.find('=') {
                if line[..eq].trim().to_lowercase() == key_l {
                    out.push(format!("{}={}", key, value));
                    key_written = true;
                    continue;
                }
            }
        }
        out.push(line);
    }
    // 目标 section 在末尾且无该 key → 追加
    if cur == section_l && !key_written {
        out.push(format!("{}={}", key, value));
        key_written = true;
    }
    // 目标 section 不存在 → 文件末尾新建
    if !section_seen && !key_written {
        if !out.is_empty() && out.last().map(|s| !s.is_empty()).unwrap_or(false) {
            out.push(String::new());
        }
        out.push(format!("[{}]", section));
        out.push(format!("{}={}", key, value));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_ini_value_existing_section_preserves_others() {
        let content = "[Sound]\nVolume=80\n\n[Game]\nNewMove=true\n";
        let out = set_ini_value(content, "Game", "Effect", "false");
        assert!(out.contains("Effect=false"));
        assert!(out.contains("NewMove=true"));
        assert!(out.contains("Volume=80"));
    }

    #[test]
    fn test_set_ini_value_replace_key() {
        let content = "[Game]\nEffect=true\n";
        let out = set_ini_value(content, "Game", "Effect", "false");
        assert!(out.contains("Effect=false"));
        assert!(!out.contains("Effect=true"));
        assert_eq!(out.matches("Effect=").count(), 1);
    }

    #[test]
    fn test_set_ini_value_new_section() {
        let content = "[Sound]\nVolume=50\n";
        let out = set_ini_value(content, "Filter", "FilterNormalChat", "true");
        assert!(out.contains("[Filter]"));
        assert!(out.contains("FilterNormalChat=true"));
        assert!(out.contains("Volume=50"));
    }

    #[test]
    fn test_ini_read_helpers() {
        let content = "[Sound]\nVolume=30\nMusic=70\n\n[Game]\nNewMove=false\n";
        assert_eq!(ini_bool(content, "Game", "NewMove", true), false);
        assert_eq!(ini_bool(content, "Game", "Missing", true), true);
        assert_eq!(ini_percent(content, "Sound", "Volume", 0.0), 0.3);
        assert_eq!(ini_percent(content, "Sound", "Missing", 0.5), 0.5);
        assert_eq!(ini_str(content, "sound", "music"), Some("70"));
    }
}
