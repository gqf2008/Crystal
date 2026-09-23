// 回城复活落点（C# `PlayerObject.TownRevive` → `Teleport(bindMap, bindX, bindY)`）
//
// 为什么单独一个文件 + 纯函数：2026-09-23 玩家视角验收（② 跨图+复活闭环）挖出一个真实缺陷——
// 复活链路只应用了 x/y，**丢掉了 map_index**，于是绑定点在别的地图时，玩家被放到
// 「死亡地图上的绑定点坐标」：实测绑定点 map1('0')@(288,616)、死亡点 map2('2')@(500,485)，
// 复活后落在 map2@(288,616)（坐标准确、地图不对）。
// 把「落点地图来自绑定点」抽成可单测的纯函数，门禁才有地方钉（见本文件 tests）。

/// 复活落点：绑定点有效时返回 (绑定图, 绑定坐标)，否则退回当前图安全区。
pub(crate) fn revive_destination(
    current_map: u16,
    bind: Option<(u16, i32, i32)>,
    fallback: (i32, i32),
) -> (u16, i32, i32) {
    match bind {
        Some((map_index, x, y)) => (map_index, x, y),
        None => (current_map, fallback.0, fallback.1),
    }
}

#[cfg(test)]
mod tests {
    use super::revive_destination;

    /// ② 复活门禁：绑定点有效时**地图必须来自绑定点**（与当前图不同也要切过去）。
    ///
    /// 阳性对照（实做）：把 Some 分支改成 `(current_map, x, y)`
    /// （只搬坐标不换图，就是本缺陷的原形）→ 本测试立即红。
    #[test]
    fn revive_destination_takes_bind_map_not_current_map() {
        let (m, x, y) = revive_destination(2, Some((1, 288, 616)), (10, 10));
        assert_eq!((m, x, y), (1, 288, 616), "必须回到绑定点地图与坐标");
        assert_ne!(m, 2, "绝不能停留在死亡地图");

        let (m2, x2, y2) = revive_destination(2, None, (33, 44));
        assert_eq!((m2, x2, y2), (2, 33, 44), "无绑定时才用当前图安全区");
    }
}
