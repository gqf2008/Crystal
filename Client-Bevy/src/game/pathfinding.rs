//! A* 寻路（参考 Client-Macroquad/src/systems/logic/physics/pathfinding_system.rs）
//! 在 walkable 网格上计算路径，8 方向，返回瓦片坐标序列（不含起点）。

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::map_renderer::LoadedMap;

/// 寻路节点
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node {
    x: i32,
    y: i32,
    g: u32,
    f: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // 小顶堆：f 小的优先（BinaryHeap 是最大堆，反向比较）
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const DIRS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn heuristic(x: i32, y: i32, tx: i32, ty: i32) -> u32 {
    // 八方向距离（切比雪夫），对角步长 14、直步长 10 近似
    let dx = (x - tx).unsigned_abs();
    let dy = (y - ty).unsigned_abs();
    let d = dx.max(dy);
    let s = dx.min(dy);
    s * 14 + (d - s) * 10
}

/// 在 `map` 上从 `from` 寻路到 `to`。
/// 返回路径瓦片序列（不含起点，含终点）。不可达或超长返回 None。
pub fn find_path(map: &LoadedMap, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    if from == to {
        return Some(Vec::new());
    }
    if !map.in_bounds(to.0, to.1) || !map.is_walkable(to.0, to.1) {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), u32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    g_score.insert(from, 0);
    open.push(Node {
        x: from.0,
        y: from.1,
        g: 0,
        f: heuristic(from.0, from.1, to.0, to.1),
    });

    // 步数上限按地图大小自适应（700x700 绕行可能超过 2000 节点，
    // 固定 2000 会把大量可达路径误判为不可达——#27）
    let max_steps = (map.width as usize * map.height as usize).clamp(1, 2_000_000);
    let mut visited = 0usize;

    while let Some(node) = open.pop() {
        visited += 1;
        if visited > max_steps {
            return None;
        }
        if (node.x, node.y) == to {
            // 回溯路径
            let mut path = Vec::new();
            let mut cur = (node.x, node.y);
            while let Some(prev) = came_from.get(&cur) {
                path.push(cur);
                cur = *prev;
            }
            path.reverse();
            // 直线可达优先（对角优先）：A* 展开顺序可能先横后斜，路线偏离鼠标指向（#95）
            if let Some(straight) = straight_path(map, from, to) {
                return Some(straight);
            }
            // 路径平滑：把 (1,0)+(0,1) 等直线对合成 (1,1) 对角，消除 45° 锯齿
            return Some(smooth_path(map, from, path));
        }

        for (dx, dy) in DIRS {
            let nx = node.x + dx;
            let ny = node.y + dy;
            if !map.in_bounds(nx, ny) || !map.is_walkable(nx, ny) {
                continue;
            }
            // 禁止斜穿墙（两个直向格子都要可走）
            if dx != 0
                && dy != 0
                && (!map.is_walkable(node.x + dx, node.y) || !map.is_walkable(node.x, node.y + dy))
            {
                continue;
            }
            let step_cost = if dx != 0 && dy != 0 { 14 } else { 10 };
            let tentative = node.g + step_cost;
            if tentative < *g_score.get(&(nx, ny)).unwrap_or(&u32::MAX) {
                g_score.insert((nx, ny), tentative);
                came_from.insert((nx, ny), (node.x, node.y));
                open.push(Node {
                    x: nx,
                    y: ny,
                    g: tentative,
                    f: tentative + heuristic(nx, ny, to.0, to.1),
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_map(w: i32, h: i32, obstacles: &[(i32, i32)]) -> LoadedMap {
        let mut walkable = vec![vec![true; h as usize]; w as usize];
        for (x, y) in obstacles {
            walkable[*x as usize][*y as usize] = false;
        }
        LoadedMap {
            name: "test".into(),
            width: w,
            height: h,
            walkable,
        }
    }

    #[test]
    fn test_find_path_straight_line() {
        let map = test_map(10, 10, &[]);
        let path = find_path(&map, (0, 0), (5, 0)).unwrap();
        assert_eq!(path.len(), 5);
        assert_eq!(path.last(), Some(&(5, 0)));
        // 每步只走一格
        for pair in path.windows(2) {
            let dx = (pair[1].0 - pair[0].0).abs();
            let dy = (pair[1].1 - pair[0].1).abs();
            assert!(dx <= 1 && dy <= 1 && dx + dy >= 1);
        }
    }

    #[test]
    fn test_find_path_around_obstacle() {
        let map = test_map(10, 10, &[(5, 0), (5, 1), (5, 2)]);
        let path = find_path(&map, (3, 0), (7, 0)).unwrap();
        assert!(path.iter().all(|p| map.is_walkable(p.0, p.1)));
        assert_eq!(path.last(), Some(&(7, 0)));
    }

    #[test]
    fn test_find_path_unreachable() {
        let map = test_map(5, 5, &[]);
        // 起点被墙围住
        let mut blocked = vec![vec![true; 5]; 5];
        for cell in blocked[2].iter_mut() {
            *cell = false;
        }
        let map2 = LoadedMap {
            name: "t".into(),
            width: 5,
            height: 5,
            walkable: blocked,
        };
        let _ = map;
        assert!(find_path(&map2, (0, 2), (4, 2)).is_none());
    }

    #[test]
    fn test_same_tile_returns_empty() {
        let map = test_map(5, 5, &[]);
        assert_eq!(find_path(&map, (2, 2), (2, 2)).unwrap().len(), 0);
    }

    /// 斜穿墙角规则：对角邻格被正交格挡住时不允许斜切（macroquad 同款）
    #[test]
    fn test_corner_cut_rejected() {
        // 右侧 (3,2) 是障碍 → 从 (2,2) 到 (3,3) 不能直接斜切，必须先正交走到 (2,3)
        let map = test_map(6, 6, &[(3, 2)]);
        let p = find_path(&map, (2, 2), (3, 3)).unwrap();
        assert_eq!(p[0], (2, 3), "右格障碍时第一步应向下而非斜切");
        // 两个正交格都可走 → 允许一步对角
        let map3 = test_map(6, 6, &[]);
        let p = find_path(&map3, (2, 2), (3, 3)).unwrap();
        assert_eq!(p, vec![(3, 3)]);
        // 左/右都堵死、下方可走 → 绕行，不斜切
        let map4 = test_map(6, 6, &[(3, 2), (1, 2)]);
        let p = find_path(&map4, (2, 2), (3, 3)).unwrap();
        assert!(p.windows(2).all(|w| {
            let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
            !(dx != 0 && dy != 0) || (map4.is_walkable(w[0].0 + dx, w[0].1) && map4.is_walkable(w[0].0, w[0].1 + dy))
        }), "路径中不允许斜穿墙角");
    }

    /// 纯 45° 直线路径应保持单一对角方向（smooth_path 不应合成锯齿）
    #[test]
    fn test_diagonal_path_stays_diagonal() {
        let map = test_map(20, 20, &[]);
        let p = find_path(&map, (2, 2), (8, 8)).unwrap();
        assert_eq!(p.len(), 6);
        for (i, node) in p.iter().enumerate() {
            assert_eq!(*node, (2 + i as i32 + 1, 2 + i as i32 + 1), "应为纯对角");
        }
    }

    /// 大图（700x700）寻路性能：开放地图对角寻路应远快于 500ms
    #[test]
    fn test_large_map_perf() {
        let map = test_map(700, 700, &[]);
        let t0 = std::time::Instant::now();
        let p = find_path(&map, (0, 0), (699, 699)).unwrap();
        let dt = t0.elapsed();
        assert_eq!(p.len(), 699);
        assert!(
            dt.as_secs_f64() < 0.5,
            "700x700 开放地图寻路应 < 500ms，实际 {:?}",
            dt
        );
    }

    /// 绕障碍时方向应稳定：路径中 45° 对角步占多数（不来回横跳）
    #[test]
    fn test_around_obstacle_uses_diagonals() {
        // 障碍列挡路，玩家需绕行；路径应尽量使用对角步保持方向
        let map = test_map(12, 12, &[(6, 2), (6, 3), (6, 4), (6, 5), (6, 6)]);
        let p = find_path(&map, (2, 2), (9, 7)).unwrap();
        assert!(p.iter().all(|n| map.is_walkable(n.0, n.1)));
        assert_eq!(p.last(), Some(&(9, 7)));
        // 计算方向变化次数：相邻两步方向不同的次数应很少（<=1 或 2）
        let dirs: Vec<(i32, i32)> = p
            .iter()
            .scan((2, 2), |prev, n| {
                let d = (n.0 - prev.0, n.1 - prev.1);
                *prev = *n;
                Some(d)
            })
            .collect();
        let changes = dirs.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(changes <= 2, "绕障碍方向变化应稳定，实际 {} 次：{:?}", changes, dirs);
    }
}


/// 路径平滑：将锯齿状直线对 (1,0)+(0,1) 或 (0,1)+(1,0) 合并为对角步 (1,1)，
/// 让 45° 斜向移动保持单一方向（消除 A* 等代价路径的任意 tie-break 造成的抖动）。
/// 直线可达检测：从 from 到 to 沿"对角优先"直线每步检查可走（含防斜穿墙）。
/// 用于空地对角移动——A* 的展开顺序可能产生"先横后斜"路径，玩家路线呈 L 形
/// 偏离鼠标指向；直线路径让玩家直接朝目标方向走（#95）。
fn straight_path(map: &LoadedMap, from: (i32, i32), to: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    if dx == 0 && dy == 0 {
        return Some(Vec::new());
    }
    let sx = dx.signum();
    let sy = dy.signum();
    let diag = dx.abs().min(dy.abs());
    let mut path = Vec::with_capacity((dx.abs() + dy.abs()) as usize);
    let mut cur = from;
    // 对角段优先（朝目标方向）
    for _ in 0..diag {
        let nx = cur.0 + sx;
        let ny = cur.1 + sy;
        if !map.in_bounds(nx, ny)
            || !map.is_walkable(nx, ny)
            || !map.is_walkable(cur.0 + sx, cur.1)
            || !map.is_walkable(cur.0, cur.1 + sy)
        {
            return None;
        }
        cur = (nx, ny);
        path.push(cur);
    }
    // 剩余水平/垂直段
    for _ in 0..(dx.abs() - diag) {
        let nx = cur.0 + sx;
        if !map.in_bounds(nx, cur.1) || !map.is_walkable(nx, cur.1) {
            return None;
        }
        cur = (nx, cur.1);
        path.push(cur);
    }
    for _ in 0..(dy.abs() - diag) {
        let ny = cur.1 + sy;
        if !map.in_bounds(cur.0, ny) || !map.is_walkable(cur.0, ny) {
            return None;
        }
        cur = (cur.0, ny);
        path.push(cur);
    }
    if cur == to {
        Some(path)
    } else {
        None
    }
}

fn smooth_path(map: &LoadedMap, from: (i32, i32), path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut out: Vec<(i32, i32)> = Vec::with_capacity(path.len());
    let mut prev = from;
    let mut i = 0;
    while i < path.len() {
        let n = path[i];
        let dx = n.0 - prev.0;
        let dy = n.1 - prev.1;
        if (dx == 0 || dy == 0) && (dx.abs() + dy.abs() == 1) && i + 1 < path.len() {
            let n2 = path[i + 1];
            let dx2 = n2.0 - prev.0;
            let dy2 = n2.1 - prev.1;
            if dx2.abs() == 1 && dy2.abs() == 1 {
                // 合成对角：检查对角格与两个直向格都可走（防斜穿墙）
                let corner_ok = map.is_walkable(n2.0, n2.1)
                    && map.is_walkable(prev.0 + dx2, prev.1)
                    && map.is_walkable(prev.0, prev.1 + dy2);
                if corner_ok {
                    out.push(n2);
                    prev = n2;
                    i += 2;
                    continue;
                }
            }
        }
        out.push(n);
        prev = n;
        i += 1;
    }
    out
}
