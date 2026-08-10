//! Boss 共用 helper 函数（对齐 C# FindAllTargets/PoisonTarget/Broadcast 等）

/// #1391：C# 红名阈值（PlayerObject.cs:250：PKPoints >= 200）
pub const RED_NAME_PK: i32 = 200;

/// #1391：是否红名（C# PKPoints >= 200；守卫/城镇弓箭手目标判定用）
pub fn is_red_name(pk_points: i32) -> bool {
    pk_points >= RED_NAME_PK
}
/// #1828：选“最弱”玩家目标（C# DarkCaptain/SnowWolfKing 按 MinDC 选更弱目标）。
/// 返回 MinDC 最低的玩家；空列表返回 None。
pub fn weakest_player_by_dc(targets: &[crate::actors::world::ai::ctx::PlayerSnap]) -> Option<crate::actors::world::ai::ctx::PlayerSnap> {
    targets.iter().min_by_key(|p| p.min_dc).copied()
}


/// 方向增量（8 方向，对齐 MirDirection）
pub const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
pub const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

/// C# HalfmoonAttack/ThreeQuarterMoonAttack：从 PreviousDir(direction) 起连续 count 个方向、距离 1 的弧形格
/// （对齐 Server/MirObjects/MonsterObject.cs:3715：dir = PreviousDir(Direction)；循环 count 次 PointMove(...,1) + NextDir）
pub fn arc_cells(center_x: i32, center_y: i32, direction: u8, count: u8) -> Vec<(i32, i32)> {
    let mut cells = Vec::with_capacity(count as usize);
    let mut d = (direction as i32 + 7).rem_euclid(8) as usize; // Functions.PreviousDir
    for _ in 0..count {
        cells.push((center_x + DIR_DX[d], center_y + DIR_DY[d]));
        d = (d + 1) % 8; // Functions.NextDir
    }
    cells
}

/// C# TriangleAttack（Server/MirObjects/MonsterObject.cs:3540）：沿 direction 每行 center + Left/Right 扩展。
/// Left(d)=(d+6)%8、Right(d)=(d+2)%8（Shared/Functions/Functions.cs:248/280）；limit_width=-1 不限单侧格数。
pub fn triangle_cells(center_x: i32, center_y: i32, direction: u8, distance: u8, limit_width: i32) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    let d = (direction as usize) % 8;
    let (cdx, cdy) = (DIR_DX[d], DIR_DY[d]);
    let (ldx, ldy) = (DIR_DX[(d + 6) % 8], DIR_DY[(d + 6) % 8]);
    let (rdx, rdy) = (DIR_DX[(d + 2) % 8], DIR_DY[(d + 2) % 8]);
    for i in 1..=distance as i32 {
        let cx = center_x + cdx * i;
        let cy = center_y + cdy * i;
        cells.push((cx, cy));
        if distance > 1 {
            let offset = i - 1;
            for k in 1..=offset {
                if limit_width >= 0 && k > limit_width {
                    break;
                }
                cells.push((cx + ldx * k, cy + ldy * k));
            }
            for k in 1..=offset {
                if limit_width >= 0 && k > limit_width {
                    break;
                }
                cells.push((cx + rdx * k, cy + rdy * k));
            }
        }
    }
    cells
}

/// C# IceThrust（Kirin.cs:126 / ManectricClaw.cs:85）：3 列（prevdir/dir/nextdir 起点）× 3 深 = 9 格。
/// 返回 (x, y, j)，j=0..depth-1 为纵深段（ManectricClaw 用 j<=1 近 DC / j==2 远 MC）。
pub fn ice_thrust_cells(center_x: i32, center_y: i32, direction: u8, depth: u8) -> Vec<(i32, i32, i32)> {
    let mut cells = Vec::new();
    let d = (direction as usize) % 8;
    let (dx, dy) = (DIR_DX[d], DIR_DY[d]);
    for col in [((d + 7) % 8), d, ((d + 1) % 8)] {
        let (sx, sy) = (center_x + DIR_DX[col], center_y + DIR_DY[col]);
        for j in 0..depth as i32 {
            cells.push((sx + dx * j, sy + dy * j, j));
        }
    }
    cells
}

/// C# ExplosionDie（HumanAssassin.cs:296）/ FullmoonAttack distance>1（MonsterObject.cs:3795、DarkOmaKing.cs:110）：8 方向 × 1..=max_radius 圈（i%8 方向、i/8+1 距离）。
pub fn eight_dir_rings(center_x: i32, center_y: i32, max_radius: u8) -> Vec<(i32, i32)> {
    let mut cells = Vec::new();
    for r in 1..=max_radius as i32 {
        for d in 0..8usize {
            cells.push((center_x + DIR_DX[d] * r, center_y + DIR_DY[d] * r));
        }
    }
    cells
}

/// 计算朝向目标的 8 方向（对齐 C# DirectionFromPoint）
pub fn direction_towards(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> u8 {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    if dx == 0 && dy == 0 {
        return 0;
    }
    // 8 方向：用角度映射
    let angle = (dy as f32).atan2(dx as f32) * 180.0 / std::f32::consts::PI;
    // MirDirection: Up=0, UpRight=1, Right=2, DownRight=3, Down=4, DownLeft=5, Left=6, UpLeft=7
    // atan2: 右=0, 下=90, 左=180, 上=-90
    let normalized = ((angle + 360.0) % 360.0 + 22.5) as i32 % 360 / 45;
    // 映射：0=右(2), 45=下右(3), 90=下(4), 135=下左(5), 180=左(6), 225=上左(7), 270=上(0), 315=上右(1)
    match normalized {
        0 => 2, // 右
        1 => 3, // 下右
        2 => 4, // 下
        3 => 5, // 下左
        4 => 6, // 左
        5 => 7, // 上左
        6 => 0, // 上
        7 => 1, // 上右
        _ => 0,
    }
}

/// 朝目标方向走一步（贪心，返回新位置和方向）
pub fn step_toward(from_x: i32, from_y: i32, tx: i32, ty: i32) -> (i32, i32, u8) {
    let mut best_dir = 4u8;
    let mut best_dist = ((tx - from_x).pow(2) + (ty - from_y).pow(2)) as i64;
    for dir in 0..8u8 {
        let nx = from_x + DIR_DX[dir as usize];
        let ny = from_y + DIR_DY[dir as usize];
        let dist = ((tx - nx).pow(2) + (ty - ny).pow(2)) as i64;
        if dist < best_dist {
            best_dist = dist;
            best_dir = dir;
        }
    }
    (
        from_x + DIR_DX[best_dir as usize],
        from_y + DIR_DY[best_dir as usize],
        best_dir,
    )
}

/// 远离目标走一步（逃跑用）
pub fn step_away(from_x: i32, from_y: i32, tx: i32, ty: i32) -> (i32, i32, u8) {
    step_toward(
        from_x,
        from_y,
        from_x + (from_x - tx),
        from_y + (from_y - ty),
    )
}

/// 切比雪夫距离（对齐 C# MaxDistance）
pub fn max_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (x1 - x2).abs().max((y1 - y2).abs())
}

/// 曼哈顿距离（对齐 C# InRange 的部分用法）
pub fn manhattan(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (x1 - x2).abs() + (y1 - y2).abs()
}

/// #1441：C# SpawnSlaves 数量 = min(requested, max(0, cap_total - SlaveList.Count))
pub fn slave_spawn_count(requested: usize, slave_count: usize, cap_total: i32) -> usize {
    requested.min((cap_total - slave_count as i32).max(0) as usize)
}

#[cfg(test)]
mod tests {
    use super::arc_cells;
    use super::eight_dir_rings;
    use super::ice_thrust_cells;
    use super::triangle_cells;
    use super::slave_spawn_count;

    #[test]
    fn slave_spawn_count_caps_at_slave_list_room() {
        // #1441：C# min(n, cap - SlaveList.Count)，负数钳 0
        assert_eq!(slave_spawn_count(8, 0, 40), 8);
        assert_eq!(slave_spawn_count(8, 38, 40), 2);
        assert_eq!(slave_spawn_count(8, 40, 40), 0);
        assert_eq!(slave_spawn_count(8, 45, 40), 0); // 已超上限不再召
        assert_eq!(slave_spawn_count(3, 4, 6), 2); // MeowMeow 6-4=2
        assert_eq!(slave_spawn_count(1, 3, 4), 1); // HoodedSummoner 4-3=1
        assert_eq!(slave_spawn_count(6, 35, 40), 5); // AncientBringer 40-35=5
        assert_eq!(slave_spawn_count(8, 25, 30), 5); // TurtleKing 30-25=5
    }
    #[test]
    fn triangle_cells_matches_csharp() {
        // C# TriangleAttack(d, 3, 1) dir=0(Up)：row1 (0,-1)；row2 (0,-2),(-1,-2),(1,-2)；row3 (0,-3),(-1,-3),(1,-3)
        let cells = triangle_cells(0, 0, 0, 3, 1);
        assert_eq!(cells.len(), 7);
        for c in [(0, -1), (0, -2), (-1, -2), (1, -2), (0, -3), (-1, -3), (1, -3)] {
            assert!(cells.contains(&c), "missing {c:?}");
        }
        // C# TriangleAttack(d, 3, 2)：row3 左右扩展到 k=2 → 9 格
        let cells = triangle_cells(0, 0, 0, 3, 2);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(-2, -3)));
        assert!(cells.contains(&(2, -3)));
        // C# TriangleAttack(d, 2, 1)：4 格
        let cells = triangle_cells(0, 0, 0, 2, 1);
        assert_eq!(cells.len(), 4);
        // dir=2（Right）：row i=(i,0)，left=(0,-1)*k、right=(0,1)*k
        let cells = triangle_cells(0, 0, 2, 2, 1);
        assert_eq!(cells.len(), 4);
        for c in [(1, 0), (2, 0), (2, -1), (2, 1)] {
            assert!(cells.contains(&c), "missing {c:?}");
        }
    }

    #[test]
    fn triangle_cells_width_limit_and_no_dupes() {
        for dir in 0..8u8 {
            let cells = triangle_cells(10, 10, dir, 3, 1);
            assert_eq!(cells.len(), 7, "dir={dir} 宽1 应 7 格");
            let mut sorted = cells.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), cells.len(), "dir={dir} 不应重复");
            // 对角方向左/右扩是斜向（如 UpRight 的 Left=UpLeft），最大切比雪夫距离 = distance + width
            for &(x, y) in &cells {
                assert!((x - 10).abs().max((y - 10).abs()) <= 4, "dir={dir} 超距 {x},{y}");
            }
        }
    }

    #[test]
    fn ice_thrust_cells_matches_csharp() {
        // C# Kirin/ManectricClaw.IceThrust：dir=0(Up)，3 列（UpLeft/Up/UpRight 起点）× 3 深 = 9 格
        let cells = ice_thrust_cells(0, 0, 0, 3);
        assert_eq!(cells.len(), 9);
        for c in [(-1, -1, 0), (-1, -2, 1), (-1, -3, 2), (0, -1, 0), (0, -2, 1), (0, -3, 2), (1, -1, 0), (1, -2, 1), (1, -3, 2)] {
            assert!(cells.contains(&c), "missing {c:?}");
        }
        // dir=2(Right)：起点 (1,-1)/(1,0)/(1,1)，向右延伸 3 深
        let cells = ice_thrust_cells(0, 0, 2, 3);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(2, -1, 1)));
        assert!(cells.contains(&(3, 1, 2)));
        // j 值 0/1/2 各 3 个（ManectricClaw 近 DC / 远 MC 分段用）
        let js: Vec<i32> = cells.iter().map(|c| c.2).collect();
        assert_eq!(js.iter().filter(|&&j| j == 0).count(), 3);
        assert_eq!(js.iter().filter(|&&j| j == 1).count(), 3);
        assert_eq!(js.iter().filter(|&&j| j == 2).count(), 3);
    }

    #[test]
    fn eight_dir_rings_matches_csharp() {
        // C# ExplosionDie：16 格（8 方向 × 2 圈）
        let cells = eight_dir_rings(0, 0, 2);
        assert_eq!(cells.len(), 16);
        let mut sorted = cells.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 16, "不应重复");
        for &(x, y) in &cells {
            assert!([1, 2].contains(&x.abs().max(y.abs())), "距离应为 1/2: {x},{y}");
        }
        assert_eq!(cells.iter().filter(|&&(x, y)| x.abs().max(y.abs()) == 1).count(), 8);
        assert_eq!(cells.iter().filter(|&&(x, y)| x.abs().max(y.abs()) == 2).count(), 8);
    }

    #[test]
    fn arc_cells_halfmoon_matches_csharp() {
        // C# HalfmoonAttack dir=0（Up）：PreviousDir=7(UpLeft) 起 4 向 → (-1,-1),(0,-1),(1,-1),(1,0)
        assert_eq!(arc_cells(0, 0, 0, 4), vec![(-1, -1), (0, -1), (1, -1), (1, 0)]);
        // dir=4（Down）：PreviousDir=3(DownRight) 起 4 向
        assert_eq!(arc_cells(0, 0, 4, 4), vec![(1, 1), (0, 1), (-1, 1), (-1, 0)]);
        // 每格都是距离 1
        for &(x, y) in &arc_cells(0, 0, 0, 4) {
            assert_eq!(x.abs().max(y.abs()), 1);
        }
    }

    #[test]
    fn arc_cells_counts_and_no_dupes() {
        for dir in 0..8u8 {
            let full = arc_cells(10, 10, dir, 8);
            assert_eq!(full.len(), 8, "dir={dir} 满弧应 8 格");
            let mut sorted = full.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), 8, "dir={dir} 不应重复");
            assert_eq!(arc_cells(10, 10, dir, 4).len(), 4, "dir={dir} 半月应 4 格");
            assert_eq!(arc_cells(10, 10, dir, 6).len(), 6, "dir={dir} 三日月应 6 格");
        }
    }

    #[test]
    fn test_weakest_player_by_dc() {
        use super::weakest_player_by_dc;
        use crate::actors::world::ai::ctx::PlayerSnap;
        let snap = |id: u64, dc: i32| PlayerSnap {
            session_id: id, x: 0, y: 0, hp: 100, map_index: 1, object_id: id as u32, level: 30, pk_points: 0, min_dc: dc,
        };
        assert!(weakest_player_by_dc(&[]).is_none());
        let snaps = [snap(1, 50), snap(2, 20), snap(3, 80)];
        let w = weakest_player_by_dc(&snaps).unwrap();
        assert_eq!(w.session_id, 2, "应选 MinDC 最低（20）而非 hp/任意");
        // 同 MinDC 时稳定选第一个
        let snaps = [snap(1, 20), snap(2, 20)];
        assert_eq!(weakest_player_by_dc(&snaps).unwrap().session_id, 1);
    }

}
