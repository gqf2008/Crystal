//! Direction and Geometry Utility Functions
//!
//! This module contains utility functions for working with directions and points
//! in the game world. These are critical shared functions used by both client and server
//! for movement calculations, position tracking, and spatial logic.
//!
//! Ported from Shared/Functions/Functions.cs

use crate::{MirDirection, Point};

/// Get the previous direction (counter-clockwise rotation)
///
/// # Example
/// ```
/// use mir2_shared::{MirDirection, utils::previous_dir};
/// assert_eq!(previous_dir(MirDirection::Up), MirDirection::UpLeft);
/// assert_eq!(previous_dir(MirDirection::Right), MirDirection::UpRight);
/// ```
pub fn previous_dir(dir: MirDirection) -> MirDirection {
    match dir {
        MirDirection::Up => MirDirection::UpLeft,
        MirDirection::UpRight => MirDirection::Up,
        MirDirection::Right => MirDirection::UpRight,
        MirDirection::DownRight => MirDirection::Right,
        MirDirection::Down => MirDirection::DownRight,
        MirDirection::DownLeft => MirDirection::Down,
        MirDirection::Left => MirDirection::DownLeft,
        MirDirection::UpLeft => MirDirection::Left,
    }
}

/// Get the next direction (clockwise rotation)
///
/// # Example
/// ```
/// use mir2_shared::{MirDirection, utils::next_dir};
/// assert_eq!(next_dir(MirDirection::Up), MirDirection::UpRight);
/// assert_eq!(next_dir(MirDirection::Right), MirDirection::DownRight);
/// ```
pub fn next_dir(dir: MirDirection) -> MirDirection {
    match dir {
        MirDirection::Up => MirDirection::UpRight,
        MirDirection::UpRight => MirDirection::Right,
        MirDirection::Right => MirDirection::DownRight,
        MirDirection::DownRight => MirDirection::Down,
        MirDirection::Down => MirDirection::DownLeft,
        MirDirection::DownLeft => MirDirection::Left,
        MirDirection::Left => MirDirection::UpLeft,
        MirDirection::UpLeft => MirDirection::Up,
    }
}

/// Calculate the direction from source point to destination point
///
/// # Example
/// ```
/// use mir2_shared::{Point, MirDirection, utils::direction_from_point};
/// let source = Point::new(10, 10);
/// let dest = Point::new(15, 10);
/// assert_eq!(direction_from_point(source, dest), MirDirection::Right);
/// ```
pub fn direction_from_point(source: Point, dest: Point) -> MirDirection {
    if source.x < dest.x {
        if source.y < dest.y {
            return MirDirection::DownRight;
        }
        if source.y > dest.y {
            return MirDirection::UpRight;
        }
        return MirDirection::Right;
    }

    if source.x > dest.x {
        if source.y < dest.y {
            return MirDirection::DownLeft;
        }
        if source.y > dest.y {
            return MirDirection::UpLeft;
        }
        return MirDirection::Left;
    }

    if source.y < dest.y {
        MirDirection::Down
    } else {
        MirDirection::Up
    }
}

/// Shift direction by N steps (positive = clockwise, negative = counter-clockwise)
///
/// # Example
/// ```
/// use mir2_shared::{MirDirection, utils::shift_direction};
/// assert_eq!(shift_direction(MirDirection::Up, 2), MirDirection::Right);
/// assert_eq!(shift_direction(MirDirection::Up, -1), MirDirection::UpLeft);
/// ```
pub fn shift_direction(dir: MirDirection, steps: i32) -> MirDirection {
    let dir_value = dir as i32;
    let new_value = ((dir_value + steps + 8) % 8) as u8;
    MirDirection::try_from(new_value).unwrap_or(dir)
}

/// Move a point in the given direction by the specified distance
///
/// # Example
/// ```
/// use mir2_shared::{Point, MirDirection, utils::point_move};
/// let p = Point::new(10, 10);
/// let moved = point_move(p, MirDirection::Right, 5);
/// assert_eq!(moved, Point::new(15, 10));
/// ```
pub fn point_move(mut point: Point, dir: MirDirection, distance: i32) -> Point {
    match dir {
        MirDirection::Up => {
            point.y -= distance;
        }
        MirDirection::UpRight => {
            point.x += distance;
            point.y -= distance;
        }
        MirDirection::Right => {
            point.x += distance;
        }
        MirDirection::DownRight => {
            point.x += distance;
            point.y += distance;
        }
        MirDirection::Down => {
            point.y += distance;
        }
        MirDirection::DownLeft => {
            point.x -= distance;
            point.y += distance;
        }
        MirDirection::Left => {
            point.x -= distance;
        }
        MirDirection::UpLeft => {
            point.x -= distance;
            point.y -= distance;
        }
    }
    point
}

/// Get the point to the left of the current position when facing the given direction
///
/// # Example
/// ```
/// use mir2_shared::{Point, MirDirection, utils::left_point};
/// let p = Point::new(10, 10);
/// let left = left_point(p, MirDirection::Up);
/// assert_eq!(left, Point::new(9, 10));
/// ```
pub fn left_point(mut point: Point, dir: MirDirection) -> Point {
    match dir {
        MirDirection::Up => {
            point.x -= 1;
        }
        MirDirection::UpRight => {
            point.x -= 1;
            point.y -= 1;
        }
        MirDirection::Right => {
            point.y -= 1;
        }
        MirDirection::DownRight => {
            point.x += 1;
            point.y -= 1;
        }
        MirDirection::Down => {
            point.x += 1;
        }
        MirDirection::DownLeft => {
            point.x += 1;
            point.y += 1;
        }
        MirDirection::Left => {
            point.y += 1;
        }
        MirDirection::UpLeft => {
            point.x -= 1;
            point.y += 1;
        }
    }
    point
}

/// Get the point to the right of the current position when facing the given direction
///
/// # Example
/// ```
/// use mir2_shared::{Point, MirDirection, utils::right_point};
/// let p = Point::new(10, 10);
/// let right = right_point(p, MirDirection::Up);
/// assert_eq!(right, Point::new(11, 10));
/// ```
pub fn right_point(mut point: Point, dir: MirDirection) -> Point {
    match dir {
        MirDirection::Up => {
            point.x += 1;
        }
        MirDirection::UpRight => {
            point.x += 1;
            point.y += 1;
        }
        MirDirection::Right => {
            point.y += 1;
        }
        MirDirection::DownRight => {
            point.x -= 1;
            point.y += 1;
        }
        MirDirection::Down => {
            point.x -= 1;
        }
        MirDirection::DownLeft => {
            point.x -= 1;
            point.y -= 1;
        }
        MirDirection::Left => {
            point.y -= 1;
        }
        MirDirection::UpLeft => {
            point.x += 1;
            point.y -= 1;
        }
    }
    point
}

/// Get the maximum distance between two points (Chebyshev distance)
///
/// This is the distance metric used in the game for range calculations.
///
/// # Example
/// ```
/// use mir2_shared::{Point, utils::max_distance};
/// let p1 = Point::new(10, 10);
/// let p2 = Point::new(15, 13);
/// assert_eq!(max_distance(p1, p2), 5);
/// ```
pub fn max_distance(p1: Point, p2: Point) -> i32 {
    std::cmp::max((p1.x - p2.x).abs(), (p1.y - p2.y).abs())
}

/// Reverse a direction (180 degree turn)
///
/// # Example
/// ```
/// use mir2_shared::{MirDirection, utils::reverse_direction};
/// assert_eq!(reverse_direction(MirDirection::Up), MirDirection::Down);
/// assert_eq!(reverse_direction(MirDirection::UpRight), MirDirection::DownLeft);
/// ```
pub fn reverse_direction(dir: MirDirection) -> MirDirection {
    match dir {
        MirDirection::Up => MirDirection::Down,
        MirDirection::UpRight => MirDirection::DownLeft,
        MirDirection::Right => MirDirection::Left,
        MirDirection::DownRight => MirDirection::UpLeft,
        MirDirection::Down => MirDirection::Up,
        MirDirection::DownLeft => MirDirection::UpRight,
        MirDirection::Left => MirDirection::Right,
        MirDirection::UpLeft => MirDirection::DownRight,
    }
}

/// Check if two points are within the specified range
///
/// Uses rectangular (Manhattan box) distance for range check.
///
/// # Example
/// ```
/// use mir2_shared::{Point, utils::in_range};
/// let a = Point::new(10, 10);
/// let b = Point::new(12, 11);
/// assert!(in_range(a, b, 2));
/// assert!(!in_range(a, b, 1));
/// ```
pub fn in_range(a: Point, b: Point, range: i32) -> bool {
    (a.x - b.x).abs() <= range && (a.y - b.y).abs() <= range
}

/// Check if two entities are facing each other
///
/// Returns true if entity A at point_a facing dir_a is looking at entity B,
/// AND entity B at point_b facing dir_b is looking at entity A.
///
/// # Example
/// ```
/// use mir2_shared::{Point, MirDirection, utils::facing_each_other};
/// let point_a = Point::new(10, 10);
/// let point_b = Point::new(12, 10);
/// assert!(facing_each_other(MirDirection::Right, point_a, MirDirection::Left, point_b));
/// assert!(!facing_each_other(MirDirection::Up, point_a, MirDirection::Left, point_b));
/// ```
pub fn facing_each_other(
    dir_a: MirDirection,
    point_a: Point,
    dir_b: MirDirection,
    point_b: Point,
) -> bool {
    dir_a == direction_from_point(point_a, point_b)
        && dir_b == direction_from_point(point_b, point_a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_rotation() {
        assert_eq!(next_dir(MirDirection::Up), MirDirection::UpRight);
        assert_eq!(next_dir(MirDirection::UpLeft), MirDirection::Up);
        assert_eq!(previous_dir(MirDirection::Up), MirDirection::UpLeft);
        assert_eq!(previous_dir(MirDirection::UpRight), MirDirection::Up);
    }

    #[test]
    fn test_reverse_direction() {
        assert_eq!(reverse_direction(MirDirection::Up), MirDirection::Down);
        assert_eq!(
            reverse_direction(MirDirection::UpRight),
            MirDirection::DownLeft
        );
        assert_eq!(reverse_direction(MirDirection::Right), MirDirection::Left);
    }

    #[test]
    fn test_shift_direction() {
        assert_eq!(shift_direction(MirDirection::Up, 2), MirDirection::Right);
        assert_eq!(shift_direction(MirDirection::Up, -1), MirDirection::UpLeft);
        assert_eq!(shift_direction(MirDirection::Right, 4), MirDirection::Left);
    }

    #[test]
    fn test_direction_from_point() {
        let center = Point::new(10, 10);
        assert_eq!(
            direction_from_point(center, Point::new(15, 10)),
            MirDirection::Right
        );
        assert_eq!(
            direction_from_point(center, Point::new(5, 10)),
            MirDirection::Left
        );
        assert_eq!(
            direction_from_point(center, Point::new(10, 15)),
            MirDirection::Down
        );
        assert_eq!(
            direction_from_point(center, Point::new(10, 5)),
            MirDirection::Up
        );
        assert_eq!(
            direction_from_point(center, Point::new(15, 5)),
            MirDirection::UpRight
        );
    }

    #[test]
    fn test_point_move() {
        let p = Point::new(10, 10);
        assert_eq!(point_move(p, MirDirection::Right, 5), Point::new(15, 10));
        assert_eq!(point_move(p, MirDirection::Up, 3), Point::new(10, 7));
        assert_eq!(
            point_move(p, MirDirection::DownRight, 2),
            Point::new(12, 12)
        );
    }

    #[test]
    fn test_left_right_point() {
        let p = Point::new(10, 10);
        assert_eq!(left_point(p, MirDirection::Up), Point::new(9, 10));
        assert_eq!(right_point(p, MirDirection::Up), Point::new(11, 10));
        assert_eq!(left_point(p, MirDirection::Right), Point::new(10, 9));
        assert_eq!(right_point(p, MirDirection::Right), Point::new(10, 11));
    }

    #[test]
    fn test_max_distance() {
        assert_eq!(max_distance(Point::new(10, 10), Point::new(15, 13)), 5);
        assert_eq!(max_distance(Point::new(0, 0), Point::new(5, 3)), 5);
        assert_eq!(max_distance(Point::new(0, 0), Point::new(3, 5)), 5);
    }

    #[test]
    fn test_in_range() {
        let a = Point::new(10, 10);
        let b = Point::new(12, 11);
        assert!(in_range(a, b, 2));
        assert!(!in_range(a, b, 1));
        assert!(in_range(a, Point::new(11, 11), 1));
    }

    #[test]
    fn test_facing_each_other() {
        let point_a = Point::new(10, 10);
        let point_b = Point::new(12, 10);
        assert!(facing_each_other(
            MirDirection::Right,
            point_a,
            MirDirection::Left,
            point_b
        ));
        assert!(!facing_each_other(
            MirDirection::Up,
            point_a,
            MirDirection::Left,
            point_b
        ));
    }
}
