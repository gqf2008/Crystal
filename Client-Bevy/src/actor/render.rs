// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::map_renderer::{make_image, GameData, GameLibraries};
use crate::resources::libraries::{ArrayLibType, LibraryName};
use crate::ui::sprite_ui::{UiFont, UiImageCache};
use super::components::*;

pub(crate) fn actor_sprite_render(
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<ActorImageCache>,
    mut q: Query<(&mut Sprite, &mut Transform, &SpriteLayer)>,
) {
    for (mut sprite, mut transform, layer) in &mut q {
        let idx = layer.frame.max(0) as u32;
        let key = (layer.lib as u8, layer.slot, idx);

        let cached = match cache.map.get(&key) {
            Some(c) => Some(CachedSprite {
                handle: c.handle.clone(),
                width: c.width,
                height: c.height,
                offset_x: c.offset_x,
                offset_y: c.offset_y,
            }),
            None => match libs
                .0
                .get_array_image(layer.lib, layer.slot as usize, idx as usize)
            {
                Some(info) => {
                    if let Some(rgba) = info.rgba.clone() {
                        let handle = images.add(make_image(
                            rgba,
                            info.width.max(0) as u32,
                            info.height.max(0) as u32,
                        ));
                        let c = CachedSprite {
                            handle,
                            width: info.width.max(0) as u32,
                            height: info.height.max(0) as u32,
                            offset_x: info.offset_x as i32,
                            offset_y: info.offset_y as i32,
                        };
                        cache.map.insert(key, c);
                        Some(CachedSprite {
                            handle: cache.map[&key].handle.clone(),
                            width: cache.map[&key].width,
                            height: cache.map[&key].height,
                            offset_x: cache.map[&key].offset_x,
                            offset_y: cache.map[&key].offset_y,
                        })
                    } else {
                        None
                    }
                }
                None => None,
            },
        };

        match cached {
            Some(c) => {
                sprite.image = c.handle;
                sprite.color = Color::srgba(1.0, 1.0, 1.0, layer.alpha);
                // 相对父实体（演员脚点）的本地坐标：macroquad 里图左上角在
                // (pos.x + offset_x, pos.y + offset_y)，Bevy 以中心为锚且 y 向上
                transform.translation = Vec3::new(
                    c.offset_x as f32 + c.width as f32 / 2.0,
                    -(c.offset_y as f32 + c.height as f32 / 2.0),
                    0.0,
                );
            }
            None => {
                // #28：帧号无效 → 默认图会渲染成白块；记日志便于排查
                if std::env::var_os("ACTOR_DEBUG").is_some() {
                    tracing::warn!(
                        "[ACTOR] 无效帧 lib={} slot={} idx={}",
                        layer.lib,
                        layer.slot,
                        idx
                    );
                }
                sprite.image = Handle::default();
            }
        }
    }
}

