// Character Renderer - Renders character appearance in SelectScene
// Loads character sprites from ChrSel.lib

use crate::graphics::texture_loader::MLibrary;
use std::io;
use std::path::Path;

/// 角色外观数据
#[derive(Debug, Clone)]
pub struct CharacterAppearance {
    pub class: mir2_shared::enums::MirClass,
    pub gender: mir2_shared::enums::MirGender,
    pub frame_index: usize,  // 当前动画帧
}

/// 角色渲染器
pub struct CharacterRenderer {
    chrsel_library: Option<MLibrary>,
}

impl CharacterRenderer {
    /// 创建新的角色渲染器
    pub fn new() -> Self {
        Self {
            chrsel_library: None,
        }
    }
    
    /// 加载角色选择资源库
    pub fn load_chrsel_library<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let library = MLibrary::open(path)?;
        tracing::info!("✅ Loaded ChrSel.lib: {} images", library.count());
        self.chrsel_library = Some(library);
        Ok(())
    }
    
    /// 获取角色精灵索引
    /// 
    /// ChrSel.lib 布局 (基于 C# 代码):
    /// - 每个职业有多帧动画
    /// - 男性和女性分开
    /// - 索引计算: base_index + gender_offset + frame
    pub fn get_character_sprite_index(
        &self,
        class: mir2_shared::enums::MirClass,
        gender: mir2_shared::enums::MirGender,
        frame: usize,
    ) -> usize {
        use mir2_shared::enums::{MirClass, MirGender};
        
        // 每个职业的基础索引 (根据 C# SelectScene.cs)
        let base_index = match class {
            MirClass::Warrior => 0,
            MirClass::Wizard => 20,
            MirClass::Taoist => 40,
            MirClass::Assassin => 60,
            MirClass::Archer => 80,
        };
        
        // 性别偏移 (男性 0-9, 女性 10-19)
        let gender_offset = match gender {
            MirGender::Male => 0,
            MirGender::Female => 10,
        };
        
        // 帧索引 (循环, 最多10帧)
        let frame_index = frame % 10;
        
        base_index + gender_offset + frame_index
    }
    
    /// 加载角色精灵纹理数据
    /// 
    /// 注意: 此方法只返回图像数据，实际渲染需要在外部完成
    /// 这样可以避免生命周期问题
    pub fn load_character_sprite_data(
        &mut self,
        class: mir2_shared::enums::MirClass,
        gender: mir2_shared::enums::MirGender,
        frame: usize,
    ) -> io::Result<(crate::graphics::texture_loader::ImageInfo, Vec<u8>)> {
        let sprite_index = self.get_character_sprite_index(class, gender, frame);
        
        let library = self.chrsel_library.as_mut()
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "ChrSel.lib not loaded"
            ))?;
        
        library.load_image_data(sprite_index)
    }
    
    /// 获取角色精灵为 egui ColorImage (用于与 egui 集成)
    pub fn load_character_color_image(
        &mut self,
        class: mir2_shared::enums::MirClass,
        gender: mir2_shared::enums::MirGender,
        frame: usize,
    ) -> io::Result<(crate::graphics::texture_loader::ImageInfo, egui::ColorImage)> {
        let sprite_index = self.get_character_sprite_index(class, gender, frame);
        
        let library = self.chrsel_library.as_mut()
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "ChrSel.lib not loaded"
            ))?;
        
        library.load_color_image(sprite_index)
    }
    
    /// 获取角色精灵信息 (用于预计算尺寸)
    pub fn get_character_sprite_info(
        &mut self,
        class: mir2_shared::enums::MirClass,
        gender: mir2_shared::enums::MirGender,
        frame: usize,
    ) -> io::Result<crate::graphics::texture_loader::ImageInfo> {
        let sprite_index = self.get_character_sprite_index(class, gender, frame);
        
        let library = self.chrsel_library.as_mut()
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                "ChrSel.lib not loaded"
            ))?;
        
        library.get_image_info(sprite_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mir2_shared::enums::{MirClass, MirGender};

    #[test]
    fn test_sprite_index_warrior() {
        let renderer = CharacterRenderer::new();
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 0), 0);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 9), 9);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Female, 0), 10);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Female, 9), 19);
    }

    #[test]
    fn test_sprite_index_all_classes() {
        let renderer = CharacterRenderer::new();
        
        // 测试所有职业的基础索引
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 0), 0);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Wizard, MirGender::Male, 0), 20);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Taoist, MirGender::Male, 0), 40);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Assassin, MirGender::Male, 0), 60);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Archer, MirGender::Male, 0), 80);
        
        // 测试女性偏移
        assert_eq!(renderer.get_character_sprite_index(MirClass::Archer, MirGender::Female, 0), 90);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Archer, MirGender::Female, 9), 99);
    }

    #[test]
    fn test_frame_modulo() {
        let renderer = CharacterRenderer::new();
        
        // 帧数应该自动取模10
        assert_eq!(
            renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 10),
            0
        );
        assert_eq!(
            renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 15),
            5
        );
    }
}
