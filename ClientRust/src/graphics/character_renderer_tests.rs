// CharacterRenderer 集成测试
// 验证 ChrSel.lib 加载和精灵索引计算

#[cfg(test)]
mod character_renderer_tests {
    use crate::graphics::character_renderer::CharacterRenderer;
    use mir2_shared::enums::{MirClass, MirGender};

    #[test]
    fn test_sprite_index_calculation() {
        let renderer = CharacterRenderer::new();

        // 测试战士索引
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 0), 0);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 9), 9);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Female, 0), 10);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Female, 9), 19);

        // 测试法师索引
        assert_eq!(renderer.get_character_sprite_index(MirClass::Wizard, MirGender::Male, 0), 20);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Wizard, MirGender::Female, 0), 30);

        // 测试道士索引
        assert_eq!(renderer.get_character_sprite_index(MirClass::Taoist, MirGender::Male, 0), 40);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Taoist, MirGender::Female, 0), 50);

        // 测试刺客索引
        assert_eq!(renderer.get_character_sprite_index(MirClass::Assassin, MirGender::Male, 0), 60);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Assassin, MirGender::Female, 0), 70);

        // 测试弓手索引
        assert_eq!(renderer.get_character_sprite_index(MirClass::Archer, MirGender::Male, 0), 80);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Archer, MirGender::Female, 0), 90);
        assert_eq!(renderer.get_character_sprite_index(MirClass::Archer, MirGender::Female, 9), 99);
    }

    #[test]
    fn test_character_renderer_creation() {
        let renderer = CharacterRenderer::new();
        // 新创建的渲染器应该没有加载库
        assert!(renderer.chrsel_library.is_none());
    }

    #[test]
    #[ignore] // 需要实际的 ChrSel.lib 文件
    fn test_load_chrsel_library() {
        let mut renderer = CharacterRenderer::new();
        let result = renderer.load_chrsel_library("Data/ChrSel.lib");
        
        if std::path::Path::new("Data/ChrSel.lib").exists() {
            assert!(result.is_ok(), "Failed to load ChrSel.lib: {:?}", result);
        } else {
            assert!(result.is_err(), "Should fail if ChrSel.lib doesn't exist");
        }
    }

    #[test]
    #[ignore] // 需要实际的 ChrSel.lib 文件
    fn test_load_character_sprite() {
        let mut renderer = CharacterRenderer::new();
        
        if renderer.load_chrsel_library("Data/ChrSel.lib").is_ok() {
            // 测试加载男性战士第一帧
            let result = renderer.load_character_color_image(
                MirClass::Warrior,
                MirGender::Male,
                0
            );
            
            assert!(result.is_ok(), "Failed to load warrior sprite: {:?}", result);
            
            if let Ok((image_info, color_image)) = result {
                println!("Loaded sprite: {}x{}", image_info.width, image_info.height);
                assert!(image_info.width > 0);
                assert!(image_info.height > 0);
                assert_eq!(color_image.size[0], image_info.width as usize);
                assert_eq!(color_image.size[1], image_info.height as usize);
            }
        }
    }

    #[test]
    #[ignore] // 需要实际的 ChrSel.lib 文件
    fn test_load_all_classes() {
        let mut renderer = CharacterRenderer::new();
        
        if renderer.load_chrsel_library("Data/ChrSel.lib").is_ok() {
            let classes = [
                MirClass::Warrior,
                MirClass::Wizard,
                MirClass::Taoist,
                MirClass::Assassin,
                MirClass::Archer,
            ];
            
            let genders = [MirGender::Male, MirGender::Female];
            
            for class in &classes {
                for gender in &genders {
                    let result = renderer.load_character_color_image(*class, *gender, 0);
                    assert!(
                        result.is_ok(),
                        "Failed to load {:?} {:?} sprite",
                        class, gender
                    );
                }
            }
            
            println!("Successfully loaded all 10 character types!");
        }
    }

    #[test]
    fn test_frame_wrapping() {
        let renderer = CharacterRenderer::new();
        
        // 测试帧数超过10会正确取模
        assert_eq!(
            renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 10),
            0  // 应该回到第0帧
        );
        
        assert_eq!(
            renderer.get_character_sprite_index(MirClass::Warrior, MirGender::Male, 15),
            5  // 15 % 10 = 5
        );
    }
}
