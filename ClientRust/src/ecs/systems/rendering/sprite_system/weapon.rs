// ============================================================================
// Weapon Render Module - 武器渲染模块
// ============================================================================
//
// **职责**:
// - 渲染角色装备的武器精灵
// - 处理武器动画帧（站立、攻击）
// - 根据角色方向和动作调整武器渲染位置
//
// **数据来源**:
// - Equipment.weapon - 静态武器数据（武器ID、类型等）
// - WeaponState - 运行时状态（当前攻击类型、帧数）
// - WeaponAnimation - 动画配置（帧数、持续时间）
//
// **图形库**:
// - CWeapons[0-78] - 战士武器库
// - ARWeapons[0-26] - 弓箭手/刺客武器库
//
// **渲染流程**:
// 1. 查询所有持有武器的实体 (Equipment, WeaponState, Position, Player)
// 2. 根据职业和武器类型选择对应的武器库
// 3. 计算武器动画帧索引
// 4. 应用角色位置、方向和动作偏移
// 5. 在角色身体上层渲染武器
//
// ============================================================================

use super::SpriteRenderSystem;
use crate::ecs::components::{
    Equipment, Player, PlayerAppearance, Position, WeaponAnimation, WeaponState,
};
use crate::graphics::libraries::{get_library_from_array, LibraryArray};
use ggez::graphics::{Canvas, Color, DrawParam, GraphicsContext};
use ggez::GameResult;
use hecs::World;
use mir2_shared::enums::MirClass;

impl SpriteRenderSystem {
    /// 渲染所有武器
    ///
    /// 注意：此方法应在 character 渲染后调用，确保武器绘制在角色身体上层
    pub fn render_weapons(
        &self,
        ctx: &mut GraphicsContext,
        canvas: &mut Canvas,
        world: &World,
    ) -> GameResult {
        // 获取相机变换参数
        let (cam_x, cam_y, zoom) = Self::get_camera_transform(world).unwrap_or((0.0, 0.0, 1.0));
        let screen_width = ctx.drawable_size().0;
        let screen_height = ctx.drawable_size().1;

        // 查询所有持有武器组件的实体
        for (_entity, (pos, player, appearance, equipment, weapon_state, weapon_anim)) in world
            .query::<(
                &Position,
                &Player,
                &PlayerAppearance,
                &Equipment,
                &WeaponState,
                &WeaponAnimation,
            )>()
            .iter()
        {
            // 检查是否装备了武器
            if equipment.weapon.is_none() {
                continue;
            }

            // 计算武器精灵索引
            let frame_index = Self::calculate_weapon_frame(player, weapon_state, weapon_anim);

            // 根据职业选择武器库
            let library_array = Self::get_weapon_library(appearance.class);

            // 获取武器库
            if let Some(library) =
                get_library_from_array(library_array, weapon_anim.weapon_library_index as usize)
            {
                // 锁定库并获取纹理
                let mut lib = library.lock().unwrap();

                // 计算屏幕位置（应用相机变换）
                let (screen_x, screen_y) = Self::world_to_screen(
                    pos.x,
                    pos.y,
                    cam_x,
                    cam_y,
                    zoom,
                    screen_width,
                    screen_height,
                );

                // 获取纹理信息
                if let Ok(info) = lib.get_or_create_texture(ctx, frame_index as usize) {
                    // 应用偏移（武器图像的锚点偏移）
                    let draw_x = screen_x + (info.x as f32) * zoom;
                    let draw_y = screen_y + (info.y as f32) * zoom;

                    // 绘制武器精灵
                    if let Some(ref image) = info.image {
                        canvas.draw(
                            image,
                            DrawParam::new()
                                .dest([draw_x, draw_y])
                                .scale([zoom, zoom])
                                .color(Color::WHITE),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// 计算武器动画帧索引
    ///
    /// 武器帧计算逻辑：
    /// - 站立/行走：显示武器默认帧 (方向 * 1)
    /// - 攻击：显示攻击动画帧 (BaseIndex + 方向 * 帧数 + 当前帧)
    fn calculate_weapon_frame(
        player: &Player,
        weapon_state: &WeaponState,
        weapon_anim: &WeaponAnimation,
    ) -> i32 {

        // 如果正在攻击，显示攻击动画
        if weapon_state.is_attacking {
            // 获取当前攻击类型的帧数
            let frame_count = weapon_anim.get_attack_frames(weapon_state.current_attack);

            // 攻击动画基础索引：Attack1=0, Attack2=200, Attack3=400
            let attack_base = match weapon_state.current_attack {
                1 => 0,
                2 => 200,
                3 => 400,
                _ => 0,
            };

            // 计算索引：基础 + 方向 * 帧数 + 当前帧
            attack_base
                + (player.direction as u8 as i32 * frame_count as i32)
                + weapon_state.current_frame as i32
        } else {
            // 站立/行走：显示默认姿势（每个方向1帧）
            // 基础索引1000 + 方向
            1000 + player.direction as u8 as i32
        }
    }

    /// 获取武器库类型
    ///
    /// 根据职业返回对应的武器库数组
    fn get_weapon_library(class: MirClass) -> LibraryArray {
        match class {
            MirClass::Warrior | MirClass::Wizard | MirClass::Taoist => {
                // 战士/法师/道士 使用 CWeapons
                LibraryArray::CWeapons
            }
            MirClass::Assassin | MirClass::Archer => {
                // 刺客/弓箭手 使用 ARWeapons
                LibraryArray::ARWeapons
            }
        }
    }
}
