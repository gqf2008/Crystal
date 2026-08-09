//! Boss 共用 helper 函数（对齐 C# FindAllTargets/PoisonTarget/Broadcast 等）

/// #1391：C# 红名阈值（PlayerObject.cs:250：PKPoints >= 200）
pub const RED_NAME_PK: i32 = 200;

/// #1391：是否红名（C# PKPoints >= 200；守卫/城镇弓箭手目标判定用）
pub fn is_red_name(pk_points: i32) -> bool {
    pk_points >= RED_NAME_PK
}

/// 方向增量（8 方向，对齐 MirDirection）
pub const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
pub const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

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
