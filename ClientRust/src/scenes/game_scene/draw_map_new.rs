    fn draw_map(&self, _ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, ggez_manager: &crate::graphics::GgezManager, map: &map_control::MapControl) {
        // Get tile manager (read-only access is enough here since tiles are preloaded)
        let tile_manager = self.tile_texture_manager.borrow();
        
        // Map tile dimensions (matches C# CellWidth/CellHeight)
        const TILE_WIDTH: f32 = 48.0;
        const TILE_HEIGHT: f32 = 32.0;
        const SCALE: f32 = 2.0; // 放大2倍,让瓦片填满窗口
        
        // Calculate visible tile range
        let start_x = ((self.camera_x / SCALE) / TILE_WIDTH) as i32 - 2;
        let start_y = ((self.camera_y / SCALE) / TILE_HEIGHT) as i32 - 2;
        let end_x = (((self.camera_x / SCALE) + self.viewport_width / SCALE) / TILE_WIDTH) as i32 + 2;
        let end_y = (((self.camera_y / SCALE) + self.viewport_height / SCALE) / TILE_HEIGHT) as i32 + 2;
        
        let start_x = start_x.max(0);
        let start_y = start_y.max(0);
        let end_x = end_x.min(map.width);
        let end_y = end_y.min(map.height);
        
        use ggez::graphics::DrawParam;
        
        let mut drawn_tiles = 0;
        
        // ========== LAYER 1: BackImage (Ground) ==========
        // Only draw even coordinates (like C# does to optimize)
        for y in start_y..end_y {
            if y % 2 == 1 { continue; } // Skip odd rows
            for x in start_x..end_x {
                if x % 2 == 1 { continue; } // Skip odd columns
                
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.back_image > 0 && cell.back_index != -1 {
                        let image_index = (cell.back_image - 1) as u16;
                        
                        // Get cached texture metadata
                        if let Some(texture) = tile_manager.get_texture_from_cache(cell.back_index, image_index) {
                            // Get actual GPU texture
                            if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                // Calculate screen position with scale
                                let screen_x = (x as f32 * TILE_WIDTH * SCALE) - self.camera_x;
                                let screen_y = (y as f32 * TILE_HEIGHT * SCALE) - self.camera_y;
                                
                                // Draw tile with scale
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([screen_x, screen_y])
                                        .scale([SCALE, SCALE])
                                );
                                
                                drawn_tiles += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 2: MiddleImage (Decorations) ==========
        for y in start_y..end_y {
            for x in start_x..end_x {
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.middle_image > 0 && cell.middle_index != -1 {
                        let image_index = (cell.middle_image - 1) as u16;
                        
                        if let Some(texture) = tile_manager.get_texture_from_cache(cell.middle_index, image_index) {
                            if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                let screen_x = (x as f32 * TILE_WIDTH * SCALE) - self.camera_x;
                                let screen_y = (y as f32 * TILE_HEIGHT * SCALE) - self.camera_y;
                                
                                canvas.draw(
                                    image,
                                    DrawParam::default()
                                        .dest([screen_x, screen_y])
                                        .scale([SCALE, SCALE])
                                );
                                
                                drawn_tiles += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // ========== LAYER 3: FrontImage (Buildings, Trees) ==========
        for y in start_y..end_y {
            for x in start_x..end_x {
                if let Some(cell) = map.get_cell(x, y) {
                    if cell.front_image > 0 && cell.front_index != -1 && cell.front_index != 200 {
                        let mut image_index = (cell.front_image & 0x7FFF) - 1;
                        
                        // Handle doors (animate based on door state)
                        if let Some(_door_idx) = cell.door_index {
                            // TODO: Get door state and animate
                            // For now, just use base image
                            image_index += 0; // + (door_state.image_index + 1) * cell.door_offset
                        }
                        
                        if image_index >= 0 {
                            if let Some(texture) = tile_manager.get_texture_from_cache(cell.front_index, image_index as u16) {
                                if let Some(image) = ggez_manager.get_texture(&texture.texture_name) {
                                    let screen_x = (x as f32 * TILE_WIDTH * SCALE) - self.camera_x;
                                    let screen_y = (y as f32 * TILE_HEIGHT * SCALE) - self.camera_y;
                                    
                                    canvas.draw(
                                        image,
                                        DrawParam::default()
                                            .dest([screen_x, screen_y])
                                            .scale([SCALE, SCALE])
                                    );
                                    
                                    drawn_tiles += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Debug: show tile count
        if drawn_tiles > 0 {
            tracing::trace!("Drew {} tiles (3 layers) in range {}x{} to {}x{}", 
                drawn_tiles, start_x, start_y, end_x, end_y);
        }
    }
