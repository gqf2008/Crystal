// drawable.rs - Trait for all drawable map objects
// 所有可绘制的地图对象必须实现此trait

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use mir2_shared::Point;

/// Trait for all objects that can be drawn on the map
/// 对应C# MapObject.Draw()方法
pub trait DrawableMapObject {
    /// Draw the object at the specified location
    /// 在指定位置绘制对象
    /// 
    /// # Arguments
    /// * `ctx` - ggez context
    /// * `canvas` - Canvas to draw on
    /// * `draw_location` - Base draw location (cell position)
    fn draw(&self, ctx: &mut Context, canvas: &mut Canvas, draw_location: Point) -> GameResult;
    
    /// Get object ID
    fn object_id(&self) -> u32;
    
    /// Check if object is dead
    fn is_dead(&self) -> bool;
    
    /// Check if object is hidden
    fn is_hidden(&self) -> bool;
    
    /// Get draw priority for sorting
    /// Lower values draw first (behind), higher values draw last (in front)
    /// Items = 0, Spells = 1, Other = 2
    fn draw_priority(&self) -> i32 {
        2 // Default for most objects
    }
}
