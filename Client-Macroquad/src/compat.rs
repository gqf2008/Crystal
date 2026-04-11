// ============================================================================
// 兼容层 - macroquad 与 ggez 类型映射
// ============================================================================
//
// 此模块职责:
// 1. 提供 ggez 到 macroquad 的类型映射和适配
// 2. 游戏核心上下文 (GameContext) 管理
// 3. 常用类型重导出,简化导入
//
// 注意: 此文件仅包含必要的兼容代码,已移除未使用的占位符

// ============================================================================
// 核心类型重导出
// ============================================================================

pub use crate::network::handlers::NetworkEvent;
pub use crate::coord::{Coord, MapUtils};

/// GameResult 类型别名 (替代 ggez::GameResult)
pub type GameResult<T = ()> = Result<T, GameError>;

/// GameError 类型 (替代 ggez::GameError)
pub use crate::core::GameError;


// ============================================================================
// ggez 图形类型的 macroquad 映射
// ============================================================================

pub use macroquad::prelude::Color;

/// GraphicsContext 占位符 (macroquad不需要,仅用于trait兼容)
#[allow(dead_code)]
pub struct GraphicsContext;

impl GraphicsContext {
    pub fn drawable_size(&self) -> (f32, f32) {
        (macroquad::prelude::screen_width(), macroquad::prelude::screen_height())
    }
}

/// Canvas 占位符 (macroquad不需要,仅用于trait兼容)
#[allow(dead_code)]
pub struct Canvas;

impl Canvas {
    #[allow(dead_code)]
    pub fn draw(&mut self, _drawable: &impl std::fmt::Debug, _param: DrawParam) {
        // macroquad 使用全局绘制函数,此处为空实现
    }
}

/// DrawParam 兼容结构 (简化版)
#[derive(Debug, Clone, Copy)]
pub struct DrawParam {
    pub dest: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub color: Color,
}

impl Default for DrawParam {
    fn default() -> Self {
        Self {
            dest: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            color: macroquad::prelude::WHITE,
        }
    }
}

use macroquad::prelude::Vec2;

impl DrawParam {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dest(mut self, dest: Vec2) -> Self {
        self.dest = dest;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }
}

// ============================================================================
// 输入相关兼容
// ============================================================================

/// KeyCode 映射
pub use macroquad::prelude::KeyCode;

/// MouseButton 映射
pub use macroquad::prelude::MouseButton;

// ============================================================================
// MapLoader 和 PathFinder 占位符
// ============================================================================

/// MapLoader 占位符结构
pub struct MapLoader;

impl MapLoader {
    pub fn load_map(_world: &mut hecs::World, _reader: impl std::any::Any) -> GameResult<()> {
        // TODO: 实现地图加载
        Ok(())
    }
}

/// PathFinder 占位符结构
pub struct PathFinder {
    width: usize,
    height: usize,
    is_blocking: Box<dyn Fn(usize, usize) -> bool>,
}

impl PathFinder {
    pub fn new(
        width: usize,
        height: usize,
        is_blocking: impl Fn(usize, usize) -> bool + 'static,
    ) -> Self {
        Self {
            width,
            height,
            is_blocking: Box::new(is_blocking),
        }
    }
    
    pub fn find_path(
        &self,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Option<Vec<(usize, usize)>> {
        if start == end {
            return Some(Vec::new());
        }
        if start.0 >= self.width || start.1 >= self.height {
            return None;
        }
        if end.0 >= self.width || end.1 >= self.height {
            return None;
        }
        if (self.is_blocking)(start.0, start.1) {
            return None;
        }
        if (self.is_blocking)(end.0, end.1) {
            return None;
        }

        use std::cmp::Ordering;
        use std::collections::BinaryHeap;

        #[derive(Copy, Clone, Eq, PartialEq)]
        struct State {
            f: u32,
            g: u32,
            x: usize,
            y: usize,
        }

        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                // BinaryHeap 是最大堆，这里反转实现最小堆
                other
                    .f
                    .cmp(&self.f)
                    .then_with(|| other.g.cmp(&self.g))
                    .then_with(|| other.x.cmp(&self.x))
                    .then_with(|| other.y.cmp(&self.y))
            }
        }

        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let idx = |x: usize, y: usize| -> usize { y * self.width + x };

        let heuristic = |x: usize, y: usize, tx: usize, ty: usize| -> u32 {
            // Octile distance (8方向)
            let dx = x.abs_diff(tx);
            let dy = y.abs_diff(ty);
            let min = dx.min(dy) as u32;
            let max = dx.max(dy) as u32;
            14 * min + 10 * (max - min)
        };

        let mut open = BinaryHeap::new();
        let mut g_score = vec![u32::MAX; self.width * self.height];
        let mut came_from: Vec<Option<(usize, usize)>> = vec![None; self.width * self.height];

        let start_i = idx(start.0, start.1);
        g_score[start_i] = 0;
        open.push(State {
            f: heuristic(start.0, start.1, end.0, end.1),
            g: 0,
            x: start.0,
            y: start.1,
        });

        const DIRS: [(i32, i32, u32); 8] = [
            (0, -1, 10),
            (1, -1, 14),
            (1, 0, 10),
            (1, 1, 14),
            (0, 1, 10),
            (-1, 1, 14),
            (-1, 0, 10),
            (-1, -1, 14),
        ];

        while let Some(State { f: _f, g, x, y }) = open.pop() {
            if (x, y) == end {
                // reconstruct (exclude start)
                let mut path_rev = Vec::new();
                let mut cur = end;
                while cur != start {
                    path_rev.push(cur);
                    let prev = came_from[idx(cur.0, cur.1)]?;
                    cur = prev;
                }
                path_rev.reverse();
                return Some(path_rev);
            }

            // 过期条目
            if g != g_score[idx(x, y)] {
                continue;
            }

            for (dx, dy, cost) in DIRS {
                let nx_i = x as i32 + dx;
                let ny_i = y as i32 + dy;
                if nx_i < 0 || ny_i < 0 {
                    continue;
                }
                let nx = nx_i as usize;
                let ny = ny_i as usize;
                if nx >= self.width || ny >= self.height {
                    continue;
                }

                // 阻挡
                if (self.is_blocking)(nx, ny) {
                    continue;
                }

                // 防止“擦角”穿墙：对角移动时要求两个正交相邻格也可走
                if dx != 0 && dy != 0 {
                    let ox = (x as i32 + dx) as usize;
                    let oy = y;
                    let px = x;
                    let py = (y as i32 + dy) as usize;
                    if (self.is_blocking)(ox, oy) || (self.is_blocking)(px, py) {
                        continue;
                    }
                }

                let tentative_g = g.saturating_add(cost);
                let n_idx = idx(nx, ny);
                if tentative_g < g_score[n_idx] {
                    came_from[n_idx] = Some((x, y));
                    g_score[n_idx] = tentative_g;
                    let h = heuristic(nx, ny, end.0, end.1);
                    open.push(State {
                        f: tentative_g.saturating_add(h),
                        g: tentative_g,
                        x: nx,
                        y: ny,
                    });
                }
            }
        }

        None
    }
}

// ============================================================================
// 系统 Trait 已移到 systems/mod.rs
// ============================================================================

