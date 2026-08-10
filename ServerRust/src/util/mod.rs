pub mod admin;

/// #1805：怪物名归一化（去空格 + 小写），用于按名匹配（DB `EvilMir` ↔ 配置/脚本 `Evil Mir`）
pub fn normalized_monster_name(name: &str) -> String {
    name.to_lowercase().replace(' ', "")
}
pub mod config;
pub mod ini;
pub mod validation;
pub mod wire;
