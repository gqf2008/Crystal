//! A* 寻路（参考 Client-Macroquad/src/systems/logic/physics/pathfinding_system.rs）
//! 在 walkable 网格上计算路径，8 方向，返回瓦片坐标序列（不含起点）。

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

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
    (0, -1), (1, -1), (1, 0), (1, 1),
    (0, 1), (-1, 1), (-1, 0), (-1, -1),
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
    open.push(Node { x: from.0, y: from.1, g: 0, f: heuristic(from.0, from.1, to.0, to.1) });

    const MAX_STEPS: usize = 2000;
    let mut visited = 0usize;

    while let Some(node) = open.pop() {
        visited += 1;
        if visited > MAX_STEPS {
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
            return Some(path);
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
                && (!map.is_walkable(node.x + dx, node.y)
                    || !map.is_walkable(node.x, node.y + dy))
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
        assert!(path.iter().all(|p| !map.is_walkable(p.0, p.1) == false));
        assert_eq!(path.last(), Some(&(7, 0)));
    }

    #[test]
    fn test_find_path_unreachable() {
        let map = test_map(5, 5, &[]);
        // 起点被墙围住
        let mut blocked = vec![vec![true; 5]; 5];
        for x in 0..5 {
            blocked[2][x] = false;
        }
        let map2 = LoadedMap { name: "t".into(), width: 5, height: 5, walkable: blocked };
        let _ = map;
        assert!(find_path(&map2, (0, 2), (4, 2)).is_none());
    }

    #[test]
    fn test_same_tile_returns_empty() {
        let map = test_map(5, 5, &[]);
        assert_eq!(find_path(&map, (2, 2), (2, 2)).unwrap().len(), 0);
    }
}
