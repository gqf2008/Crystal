// ============================================================================
// Weapon Render Module - 武器渲染模块 (占位符)
// ============================================================================

use super::SpriteRenderSystem;

impl SpriteRenderSystem {
    #[allow(dead_code)]
    pub fn render_weapons(
        &mut self,
        _ctx: &mut crate::compat::GraphicsContext,
        _canvas: &mut crate::compat::Canvas,
        _world: &hecs::World,
    ) -> crate::compat::GameResult {
        // TODO: 重写为 macroquad API
        Ok(())
    }
    
    #[allow(dead_code)]
    fn get_weapon_library(_class: i32) -> crate::resources::libraries::LibraryArray {
        crate::resources::libraries::LibraryArray::CWeapons
    }
}
