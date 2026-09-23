//! P3-3（#782）：客户端「本地物品名表」的单一来源。
//!
//! 现象：仓库格 / 商城格把物品显示成 `#782` 这类内部 ID，而不是物品真名。
//!
//! 原版依据（C#，本轮实读，用来否掉「服务端补发名字」这条看似省事的路）：
//! - `Client/MirScenes/GameScene.cs:6755` `Bind(UserItem)`：线包里的 `UserItem`
//!   **不带** `ItemInfo`，名字由本地 `ItemInfoList` 按 `ItemIndex` 补；补不到就保持
//!   `null`。仓库正是这么用的——`GameScene.cs:4955` 的 `UserStorage` 处理器拿到
//!   `Storage` 后逐格 `Bind(Storage[i])`。
//! - `Client/MirScenes/GameScene.cs:168` `RequestItemInfo(int index)`：
//!   `if (index <= 0 || HasItemInfo(index) || !RequestedItemInfo.Add(index)) return;`
//!   —— 本地已有名字就不请求，同一索引只请求一次（`HashSet` 去重）。
//! - `Client/MirControls/MirScene.cs:233`：`NewItemInfo` 回包 → `ItemInfoList.Add`，
//!   即「按需请求」的回应进的是**同一张**本地表。
//! - 显示侧兜底：`Client/MirScenes/Dialogs/NPCDialogs.cs:701` 命中不到就
//!   `GameScene.RequestItemInfo(idx)` + 占位文本。
//!
//! 本端没有本地物品库，降级链因此是：
//! 条目自带名字 → 本表（`UserInformation` / `NewItemInfo` 灌入）→ **发一次**
//! `RequestItemInfo`（按索引去重）→ 兜底 `#id`。
//!
//! 仓库（`dialogs::storage`）与商城（`dialogs::game_shop`）共用本模块的同一对函数，
//! 避免两条降级链各自漂移（#782 的验收判据就是「两个界面走同一降级链」）。

use std::collections::HashMap;

/// 把 `NewItemInfo` / `UserInformation` 下发的名字写进本地表。
///
/// 空名字**不写**（否则会把表里的好名字覆盖成空串，调用方只能再回退 `#id`）。
/// 返回是否真的写入。
pub fn remember_item_name(item_names: &mut HashMap<i32, String>, index: i32, name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    item_names.insert(index, name.to_string());
    true
}

/// 显示名解析（P3-3）：`线包自带名字 → 本地物品名表 → 需要请求 → 兜底 #id`。
///
/// 返回 `(显示名, 是否需要请求物品信息)`：`true` 表示调用方**应当**按去重集合发一次
/// `RequestItemInfo`（原版 `GameScene.RequestItemInfo` 的语义，不是「显示 `#id` 就算完」）。
///
/// `#<索引>` 是本函数自己产出的**占位**，不算「自带名字」——否则占位一旦落进
/// `UserItem.name`，后来的 `NewItemInfo` 回包就再也纠正不了它（先查本地表那条链会
/// 永远失效）。把占位也当成「没有名字」，这条降级链才是自愈的。
///
/// 纯函数——单测与阳性对照都钉在它上面。
pub fn resolve_item_name(
    name: &str,
    item_names: &HashMap<i32, String>,
    item_index: i32,
) -> (String, bool) {
    let placeholder = format!("#{item_index}");
    if !name.is_empty() && name != placeholder {
        return (name.to_string(), false);
    }
    if let Some(n) = item_names.get(&item_index) {
        if !n.is_empty() {
            return (n.clone(), false);
        }
    }
    (placeholder, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 门禁（P3-3）：降级链顺序——自带名 / 本地表命中 / 两处皆无（要请求）/
    /// 表里空串同样视为「没有」（仍要请求）。
    ///
    /// 阳性对照（落地时实做）：把中间那段查表删掉（直接回 `#id`）→ 本测试立即红。
    #[test]
    fn resolve_item_name_falls_back_in_order() {
        let mut names = HashMap::new();
        names.insert(782, "马鞍".to_string());

        // ① 条目自带名字：直接用，不需要请求
        assert_eq!(
            resolve_item_name("屠龙", &names, 1268),
            ("屠龙".to_string(), false)
        );
        // ② 条目无名但本地表有：用本地表，不需要请求（#782 那一格）
        assert_eq!(
            resolve_item_name("", &names, 782),
            ("马鞍".to_string(), false)
        );
        // ③ 两处都没有：显示 #id 并**要求发起一次请求**（不是静默显示 #id 就算完）
        assert_eq!(
            resolve_item_name("", &names, 1270),
            ("#1270".to_string(), true)
        );
        // ④ 表里存了空串同样视为「没有」，仍要请求
        names.insert(1271, String::new());
        assert_eq!(
            resolve_item_name("", &names, 1271),
            ("#1271".to_string(), true)
        );
        // ⑤ 已经落成占位 `#782` 的格子：表里到货后必须被纠正成真名（自愈），
        //    否则占位会被当成「自带名字」把整条降级链闷死。
        assert_eq!(
            resolve_item_name("#782", &names, 782),
            ("马鞍".to_string(), false)
        );
    }

    /// 门禁（P3-3）：`NewItemInfo` 回包必须写进表；空名字不得覆盖已有名字。
    ///
    /// 阳性对照（落地时实做）：把 `remember_item_name` 改成直接 `return false;`
    /// （不写表）→ 本测试立即红。
    #[test]
    fn remember_item_name_fills_table_and_ignores_blank() {
        let mut names = HashMap::new();
        assert!(remember_item_name(&mut names, 782, "马鞍"));
        assert_eq!(names.get(&782).map(String::as_str), Some("马鞍"));
        // 格子侧的降级链应当立刻吃到这个名字（不再回 #id）
        assert_eq!(
            resolve_item_name("", &names, 782),
            ("马鞍".to_string(), false)
        );
        // 空名字不得覆盖已有名字
        assert!(!remember_item_name(&mut names, 782, ""));
        assert_eq!(names.get(&782).map(String::as_str), Some("马鞍"));
    }
}
