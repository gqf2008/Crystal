// PathFinder.rs - A* pathfinding algorithm
// Mirrors Client/MirObjects/PathFinder.cs

use mir2_shared::Point;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A* node for pathfinding
#[derive(Debug, Clone, Eq, PartialEq)]
struct PathNode {
    position: Point,
    g_cost: i32, // Cost from start
    h_cost: i32, // Heuristic cost to goal
    f_cost: i32, // Total cost (g + h)
    parent: Option<Point>,
}

impl PathNode {
    fn new(position: Point, g_cost: i32, h_cost: i32, parent: Option<Point>) -> Self {
        Self {
            position,
            g_cost,
            h_cost,
            f_cost: g_cost + h_cost,
            parent,
        }
    }
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap
        other.f_cost.cmp(&self.f_cost)
            .then_with(|| other.h_cost.cmp(&self.h_cost))
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinding algorithm (A*)
pub struct PathFinder {
    // Map dimensions
    width: i32,
    height: i32,
    
    // Obstacle callback
    is_blocking_fn: Box<dyn Fn(Point) -> bool>,
    
    // Performance settings
    max_iterations: usize,
    diagonal_movement: bool,
}

impl PathFinder {
    /// Create a new pathfinder
    pub fn new(
        width: i32,
        height: i32,
        is_blocking_fn: Box<dyn Fn(Point) -> bool>,
    ) -> Self {
        Self {
            width,
            height,
            is_blocking_fn,
            max_iterations: 1000,
            diagonal_movement: true,
        }
    }

    /// Find path from start to goal using A* algorithm
    pub fn find_path(&self, start: Point, goal: Point) -> Option<Vec<Point>> {
        // Validate start and goal
        if !self.is_valid_position(start) || !self.is_valid_position(goal) {
            return None;
        }

        // If goal is blocking, try to find closest valid position
        let goal = if (self.is_blocking_fn)(goal) {
            self.find_nearest_walkable(goal)?
        } else {
            goal
        };

        // If start equals goal
        if start == goal {
            return Some(vec![start]);
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<Point, Point> = HashMap::new();
        let mut g_scores: HashMap<Point, i32> = HashMap::new();

        // Initialize with start node
        let h_cost = self.heuristic(start, goal);
        open_set.push(PathNode::new(start, 0, h_cost, None));
        g_scores.insert(start, 0);

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                // Timeout - return partial path to closest node
                return self.reconstruct_partial_path(&came_from, current.position, start);
            }

            // Check if reached goal
            if current.position == goal {
                return Some(self.reconstruct_path(&came_from, goal, start));
            }

            // Add to closed set
            closed_set.insert(current.position);

            // Check all neighbors
            for neighbor in self.get_neighbors(current.position) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                // Calculate tentative g_cost
                let move_cost = if self.is_diagonal_move(current.position, neighbor) {
                    14 // Diagonal cost (sqrt(2) * 10 ≈ 14)
                } else {
                    10 // Straight cost
                };

                let tentative_g = current.g_cost + move_cost;

                // Check if this path is better
                if let Some(&existing_g) = g_scores.get(&neighbor) {
                    if tentative_g >= existing_g {
                        continue; // Not a better path
                    }
                }

                // Record this path
                came_from.insert(neighbor, current.position);
                g_scores.insert(neighbor, tentative_g);

                let h_cost = self.heuristic(neighbor, goal);
                open_set.push(PathNode::new(neighbor, tentative_g, h_cost, Some(current.position)));
            }
        }

        // No path found
        None
    }

    /// Get valid neighbors of a position
    fn get_neighbors(&self, pos: Point) -> Vec<Point> {
        let mut neighbors = Vec::new();

        // Cardinal directions (N, E, S, W)
        let cardinals = [
            Point::new(pos.x, pos.y - 1), // North
            Point::new(pos.x + 1, pos.y), // East
            Point::new(pos.x, pos.y + 1), // South
            Point::new(pos.x - 1, pos.y), // West
        ];

        for neighbor in &cardinals {
            if self.is_walkable(*neighbor) {
                neighbors.push(*neighbor);
            }
        }

        // Diagonal directions (NE, SE, SW, NW)
        if self.diagonal_movement {
            let diagonals = [
                (Point::new(pos.x + 1, pos.y - 1), 1, 0), // NE (requires E and N)
                (Point::new(pos.x + 1, pos.y + 1), 1, 2), // SE (requires E and S)
                (Point::new(pos.x - 1, pos.y + 1), 3, 2), // SW (requires W and S)
                (Point::new(pos.x - 1, pos.y - 1), 3, 0), // NW (requires W and N)
            ];

            for (diagonal, card1, card2) in &diagonals {
                // Only allow diagonal if both cardinal directions are walkable
                if self.is_walkable(cardinals[*card1]) && self.is_walkable(cardinals[*card2]) {
                    if self.is_walkable(*diagonal) {
                        neighbors.push(*diagonal);
                    }
                }
            }
        }

        neighbors
    }

    /// Check if position is walkable
    fn is_walkable(&self, pos: Point) -> bool {
        self.is_valid_position(pos) && !(self.is_blocking_fn)(pos)
    }

    /// Check if position is within map bounds
    fn is_valid_position(&self, pos: Point) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }

    /// Check if move is diagonal
    fn is_diagonal_move(&self, from: Point, to: Point) -> bool {
        let dx = (to.x - from.x).abs();
        let dy = (to.y - from.y).abs();
        dx == 1 && dy == 1
    }

    /// Heuristic function (Manhattan distance)
    fn heuristic(&self, from: Point, to: Point) -> i32 {
        let dx = (to.x - from.x).abs();
        let dy = (to.y - from.y).abs();
        
        if self.diagonal_movement {
            // Diagonal distance
            let straight = (dx - dy).abs();
            let diagonal = dx.min(dy);
            straight * 10 + diagonal * 14
        } else {
            // Manhattan distance
            (dx + dy) * 10
        }
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(&self, came_from: &HashMap<Point, Point>, mut current: Point, start: Point) -> Vec<Point> {
        let mut path = vec![current];
        
        while current != start {
            if let Some(&parent) = came_from.get(&current) {
                current = parent;
                path.push(current);
            } else {
                break;
            }
        }
        
        path.reverse();
        path
    }

    /// Reconstruct partial path (when timeout occurs)
    fn reconstruct_partial_path(&self, came_from: &HashMap<Point, Point>, mut current: Point, start: Point) -> Option<Vec<Point>> {
        let mut path = vec![current];
        
        while current != start {
            if let Some(&parent) = came_from.get(&current) {
                current = parent;
                path.push(current);
            } else {
                break;
            }
        }
        
        path.reverse();
        
        if path.len() > 1 {
            Some(path)
        } else {
            None
        }
    }

    /// Find nearest walkable position
    fn find_nearest_walkable(&self, pos: Point) -> Option<Point> {
        // Check in expanding circles
        for radius in 1..=5 {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let dx: i32 = dx;
                    let dy: i32 = dy;
                    if dx.abs() != radius && dy.abs() != radius {
                        continue; // Only check perimeter
                    }
                    
                    let check_pos = Point::new(pos.x + dx, pos.y + dy);
                    if self.is_walkable(check_pos) {
                        return Some(check_pos);
                    }
                }
            }
        }
        None
    }

    /// Smooth path (remove unnecessary waypoints)
    pub fn smooth_path(&self, path: Vec<Point>) -> Vec<Point> {
        if path.len() <= 2 {
            return path;
        }

        let mut smoothed = vec![path[0]];
        let mut current_idx = 0;

        while current_idx < path.len() - 1 {
            let mut farthest_idx = current_idx + 1;

            // Find farthest visible point
            for i in (current_idx + 2)..path.len() {
                if self.has_line_of_sight(path[current_idx], path[i]) {
                    farthest_idx = i;
                } else {
                    break;
                }
            }

            smoothed.push(path[farthest_idx]);
            current_idx = farthest_idx;
        }

        smoothed
    }

    /// Check line of sight between two points
    fn has_line_of_sight(&self, from: Point, to: Point) -> bool {
        let dx = (to.x - from.x).abs();
        let dy = (to.y - from.y).abs();
        let sx = if from.x < to.x { 1 } else { -1 };
        let sy = if from.y < to.y { 1 } else { -1 };
        let mut err = dx - dy;
        let mut current = from;

        loop {
            if current == to {
                return true;
            }

            if !(self.is_blocking_fn)(current) {
                let e2 = 2 * err;
                if e2 > -dy {
                    err -= dy;
                    current.x += sx;
                }
                if e2 < dx {
                    err += dx;
                    current.y += sy;
                }
            } else {
                return false;
            }
        }
    }

    /// Set maximum iterations (for performance)
    pub fn set_max_iterations(&mut self, max: usize) {
        self.max_iterations = max;
    }

    /// Enable/disable diagonal movement
    pub fn set_diagonal_movement(&mut self, enabled: bool) {
        self.diagonal_movement = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_map() -> Box<dyn Fn(Point) -> bool> {
        // 10x10 map with a wall at x=5
        Box::new(|pos: Point| {
            pos.x == 5 && pos.y >= 2 && pos.y <= 7 // Wall from (5,2) to (5,7)
        })
    }

    #[test]
    fn test_pathfinder_straight_line() {
        let is_blocking = Box::new(|_: Point| false); // No obstacles
        let pathfinder = PathFinder::new(10, 10, is_blocking);
        
        let start = Point::new(0, 0);
        let goal = Point::new(5, 0);
        
        let path = pathfinder.find_path(start, goal);
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
    }

    #[test]
    fn test_pathfinder_with_obstacle() {
        let pathfinder = PathFinder::new(10, 10, create_simple_map());
        
        let start = Point::new(3, 5);
        let goal = Point::new(7, 5);
        
        let path = pathfinder.find_path(start, goal);
        assert!(path.is_some());
        
        let path = path.unwrap();
        // Path should go around the wall
        assert!(!path.contains(&Point::new(5, 5)));
    }

    #[test]
    fn test_pathfinder_no_path() {
        // Complete wall blocking
        let is_blocking = Box::new(|pos: Point| pos.x == 5); // Full vertical wall
        let pathfinder = PathFinder::new(10, 10, is_blocking);
        
        let start = Point::new(3, 5);
        let goal = Point::new(7, 5);
        
        let path = pathfinder.find_path(start, goal);
        assert!(path.is_none()); // No path possible
    }

    #[test]
    fn test_path_smoothing() {
        let is_blocking = Box::new(|_: Point| false);
        let pathfinder = PathFinder::new(10, 10, is_blocking);
        
        // Zigzag path
        let path = vec![
            Point::new(0, 0),
            Point::new(1, 0),
            Point::new(2, 0),
            Point::new(3, 0),
            Point::new(4, 0),
            Point::new(5, 0),
        ];
        
        let smoothed = pathfinder.smooth_path(path);
        
        // Should be reduced to start and end
        assert!(smoothed.len() <= 3);
        assert_eq!(smoothed.first(), Some(&Point::new(0, 0)));
        assert_eq!(smoothed.last(), Some(&Point::new(5, 0)));
    }
}
