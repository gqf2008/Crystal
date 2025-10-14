use std::fmt;
use std::io::{Read, Write};
use std::ops::{Add, Sub};
use std::str::FromStr;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};

use crate::data::stats::SharedResult;

/// Basic 2D point structure (corresponds to System.Drawing.Point in C#)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Create a new point
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Read a point from binary stream
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        Ok(Self { x, y })
    }

    /// Write a point to binary stream
    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.x)?;
        writer.write_i32::<LittleEndian>(self.y)?;
        Ok(())
    }

    /// Add two points together (vector addition)
    pub fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }

    /// Add scalar values to x and y
    pub fn add_xy(self, x: i32, y: i32) -> Point {
        Point::new(self.x + x, self.y + y)
    }

    /// Subtract a point from this point (vector subtraction)
    pub fn subtract(self, other: Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }

    /// Subtract scalar values from x and y
    pub fn subtract_xy(self, x: i32, y: i32) -> Point {
        Point::new(self.x - x, self.y - y)
    }

    /// Offset this point by the given amounts (mutating)
    pub fn offset(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    /// Format as "x, y" string (matches C# PointToString)
    pub fn to_string(&self) -> String {
        format!("{}, {}", self.x, self.y)
    }
}

/// Implement Display for Point to match C# ToString behavior
impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.x, self.y)
    }
}

/// Parse a point from string format "x, y" (matches C# TryParse)
impl FromStr for Point {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid point format: '{}'. Expected 'x, y'", s));
        }

        let x = parts[0]
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("Failed to parse x coordinate: {}", e))?;
        let y = parts[1]
            .trim()
            .parse::<i32>()
            .map_err(|e| format!("Failed to parse y coordinate: {}", e))?;

        Ok(Point::new(x, y))
    }
}

/// Implement + operator for Point
impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

/// Implement - operator for Point
impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_new() {
        let p = Point::new(10, 20);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
    }

    #[test]
    fn test_point_add() {
        let p1 = Point::new(10, 20);
        let p2 = Point::new(5, 7);
        assert_eq!(p1.add(p2), Point::new(15, 27));
        assert_eq!(p1 + p2, Point::new(15, 27)); // Test operator
    }

    #[test]
    fn test_point_subtract() {
        let p1 = Point::new(10, 20);
        let p2 = Point::new(5, 7);
        assert_eq!(p1.subtract(p2), Point::new(5, 13));
        assert_eq!(p1 - p2, Point::new(5, 13)); // Test operator
    }

    #[test]
    fn test_point_add_xy() {
        let p = Point::new(10, 20);
        assert_eq!(p.add_xy(5, 7), Point::new(15, 27));
    }

    #[test]
    fn test_point_subtract_xy() {
        let p = Point::new(10, 20);
        assert_eq!(p.subtract_xy(5, 7), Point::new(5, 13));
    }

    #[test]
    fn test_point_offset() {
        let mut p = Point::new(10, 20);
        p.offset(5, 7);
        assert_eq!(p, Point::new(15, 27));
    }

    #[test]
    fn test_point_to_string() {
        let p = Point::new(10, 20);
        assert_eq!(p.to_string(), "10, 20");
        assert_eq!(format!("{}", p), "10, 20"); // Test Display trait
    }

    #[test]
    fn test_point_from_str() {
        assert_eq!("10, 20".parse::<Point>().unwrap(), Point::new(10, 20));
        assert_eq!("5,7".parse::<Point>().unwrap(), Point::new(5, 7));
        assert_eq!("  15  ,  25  ".parse::<Point>().unwrap(), Point::new(15, 25));
        assert!("invalid".parse::<Point>().is_err());
        assert!("10".parse::<Point>().is_err());
    }
}
