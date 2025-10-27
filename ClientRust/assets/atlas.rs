use crate::assets::{AssetId, Assets, Texture};
use crate::scene::game::animation::Animation;
use crate::scene::game::player::Player;
use crate::scene::LogicHandler;
use crate::tiled::{self, Map, TileId, Value};
use crate::transform::Camera2D;
use ggez::glam::{vec3, Mat4};
use ggez::graphics::Canvas;
use ggez::mint::Point2;
use ggez::GameResult;
use ggez::{
    context::HasMut,
    graphics::{
        self, BlendMode, Color, DrawParam, Drawable, InstanceArray, Mesh, Rect, ScreenImage,
    },
    Context,
};
use keyframe::functions::Linear;
use std::collections::HashMap;

use super::player::PlayerState;

//世界坐标系
//(0,0)-------->(x,0)
// |
// |
// |
// |
//(0,y)-------->(x,y)
// 定义地图结构体
pub struct Atlas {
    // 可见区域的Tile边界
    pub top: isize,
    pub left: isize,
    pub right: isize,
    pub bottom: isize,
    // 地图名称
    pub name: String,
    // 地图信息
    pub map: tiled::Map,

    textures: HashMap<TileId, Texture>,
    pub tile_offset_x: i16,
    pub tile_offset_y: i16,
    pub object_offset_x: i16,
    pub object_offset_y: i16,

    pub effect_offset_x: i16,
    pub effect_offset_y: i16,

    pub draw_tile_layer: bool,
    pub draw_smtile_layer: bool,
    pub draw_object_layer: bool,
    pub draw_effect_layer: bool,
    pub draw_collision_layer: bool,
    pub draw_grid_layer: bool,

    mesh: graphics::Mesh,
    background: ScreenImage,
    effect_layer: ScreenImage,
    foreground: ScreenImage,
    sprite_layer: ScreenImage,
    animations: HashMap<(usize, usize), Animation>,
    has_ani: bool,
    minimap: Option<Texture>,
    inited: bool,
    // assets: Assets,
}

impl Atlas {
    // 绘制地图时向左延伸块儿数量
    pub const EXTEND_LEFT: u8 = 5;
    // 绘制地图时向右延伸块儿数量
    pub const EXTEND_RIGHT: u8 = 5;
    // 绘制地图时向下延伸块儿数量
    pub const EXTEND_BOTTOM: u8 = 25;
    pub fn new(ctx: &mut Context, map: tiled::Map) -> anyhow::Result<Self> {
        let map = map.transpose(); //转置地图

        let mesh = Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(1.),
            Rect::new(0., 0., map.tile_width as f32, map.tile_height as f32),
            Color::WHITE,
        )?;
        let background = graphics::ScreenImage::new(ctx, 1., 1., 1);
        let effect_layer = graphics::ScreenImage::new(ctx, 1., 1., 1);
        let foreground = graphics::ScreenImage::new(ctx, 1., 1., 1);
        let sprite_layer = graphics::ScreenImage::new(ctx, 1., 1., 1);
    let assets = Assets::new();
        let mut textures = HashMap::new();
        for tid in map.tilesets.keys() {
            let chunk = assets.load_image(&(tid.tileset(), tid.idx()).into())?;
            chunk.and_then(|chunk| {
                textures.insert(tid.clone(), chunk.to_texture(ctx.retrieve_mut()));
                None::<()>
            });
        }
        let mut animations = HashMap::new();
        if let Some(layer) = map.layers.get("animationlayer") {
            for y in 0..map.height as usize {
                for x in 0..map.width as usize {
                    if let Some(tile) = layer.get_tile_xy(x, y) {
                        let frames = tile
                            .properties
                            .get("frames")
                            .unwrap_or(&Value::U8(0))
                            .as_i8();
                        let mut tick = tile.properties.get("tick").unwrap_or(&Value::I8(0)).as_i8();

                        if frames == 0 || frames == i8::MAX || tick == -1 {
                            continue;
                        }
                        if tick == 0 {
                            tick = 1;
                        }
                        let fps = frames as f32 / tick as f32;
                        let mut sequence: Vec<AssetId> = vec![];
                        for idx in tile.id.idx()..tile.id.idx() + frames as i16 {
                            //println!("{}.{idx}", &tile.id.tileset(),);
                            let tid: AssetId = (tile.id.tileset(), idx).into();
                            sequence.push(tid.clone());
                            let chunk = assets.load_image(&tid)?;
                            chunk.and_then(|chunk| {
                                textures.insert(tid.into(), chunk.to_texture(ctx.retrieve_mut()));
                                None::<()>
                            });
                        }
                        let ani = Animation::new(0, 0)
                            .frames(frames as usize)
                            .fps(fps as f32)
                            .sequence(sequence)
                            .easing(Linear)
                            .build();

                        //tile.id.idx()..tile.id.idx() + frames as i16;
                        animations.insert((x, y), ani);
                    }
                }
            }
        }

        let mut minimap = None;

        read_mmap(&map.name)
            .and_then(|idx| assets.load_image(&("mmap", idx - 1).into()).ok())
            .and_then(|chunk| chunk)
            .and_then(|chunk| {
                minimap = Some(chunk.to_texture(ctx.retrieve_mut()));

                None::<()>
            });
        Ok(Atlas {
            name: map.name.to_string(),
            map,
            minimap,
            inited: false,
            // assets,
            tile_offset_x: 0,
            tile_offset_y: 0,
            object_offset_x: 0,
            object_offset_y: -32,
            effect_offset_x: -54,
            effect_offset_y: -136,
            textures,
            draw_tile_layer: true,
            draw_smtile_layer: true,
            draw_object_layer: true,
            draw_effect_layer: true,
            draw_collision_layer: false,
            draw_grid_layer: false,
            mesh,
            background,
            effect_layer,
            foreground,
            sprite_layer,
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            animations,
            has_ani: false,
        })
    }
}

impl Atlas {
    pub fn map_to_world(&self, x: i32, y: i32) -> [f32; 2] {
        [
            x as f32 * self.map.tile_width as f32,
            y as f32 * self.map.tile_height as f32,
        ]
    }

    pub fn world_to_map(&self, x: f32, y: f32) -> [i32; 2] {
        [
            (x / self.map.tile_width as f32) as i32,
            (y / self.map.tile_height as f32) as i32,
        ]
    }
    fn update_data(&mut self, camera: &Camera2D) {
        let (screen_width, screen_height) = (camera.viewport_width, camera.viewport_height);
        let tile_width = self.map.tile_width as f32;
        let tile_height = self.map.tile_height as f32;
        // 屏幕四个角的坐标（屏幕坐标系）
        let screen_corners = [
            Point2 { x: 0.0, y: 0.0 },
            Point2 {
                x: screen_width,
                y: 0.0,
            },
            Point2 {
                x: screen_width,
                y: screen_height,
            },
            Point2 {
                x: 0.0,
                y: screen_height,
            },
        ];
        // 将屏幕坐标转换为世界坐标，得到可见区域的四个角
        let world_corners: Vec<Point2<f32>> = screen_corners
            .iter()
            .map(|&p| camera.screen_to_world(p))
            .collect();
        // 计算可见区域的边界
        let min_x = world_corners
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = world_corners
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = world_corners
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = world_corners
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        // 根据瓦片大小计算需要绘制的瓦片索引范围
        self.left = (min_x / tile_width).floor() as isize - 5;
        self.right = (max_x / tile_width).ceil() as isize + 5;
        self.top = (min_y / tile_height).floor() as isize - 5;
        self.bottom = (max_y / tile_height).ceil() as isize + 25;

        self.left = self.left.max(0);
        self.right = self.right.min(self.map.width as isize);
        self.top = self.top.max(0);
        self.bottom = self.bottom.min(self.map.height as isize);
        if let Some(_layer) = self.map.layers.get("animationlayer") {
            let top = self.top as usize;
            let left = self.left as usize;
            let right = self.right as usize;
            let bottom = self.bottom as usize;
            for y in top..=bottom {
                for x in left..=right {
                    if self.animations.get(&(x, y)).is_some() {
                        self.has_ani = true;
                        return;
                    }
                }
            }
            self.has_ani = false;
        }
    }

    pub fn has_animation(&self) -> bool {
        self.has_ani
    }

    pub fn get_texture(&self, tid: &TileId) -> Option<&Texture> {
        self.textures.get(tid)
    }

    pub fn draw(
        &mut self,
        ctx: &mut Context,
        canvas: &mut graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        let param: DrawParam = param.into();
        canvas.draw(&self.background.image(ctx), param);

        canvas.set_blend_mode(BlendMode::ALPHA);
        canvas.draw(&self.foreground.image(ctx), param);

        if self.has_ani {
            canvas.set_blend_mode(BlendMode::ADD);
            canvas.draw(&self.effect_layer.image(ctx), param);
        }
        // canvas.set_blend_mode(BlendMode::ADD);
        // canvas.draw(&self.sprite_layer.image(ctx), param.z(1));
    }

    pub fn redraw(&mut self, ctx: &mut Context, actor: &Player, camera: &Camera2D) -> GameResult {
        if actor.state() == PlayerState::Run || actor.state() == PlayerState::Walk || !self.inited {
            //重新绘制地图背景
            let mut canvas = Canvas::from_screen_image(ctx, &mut self.background, Color::BLACK);
            self.draw_background(&mut canvas, camera)?;
            canvas.finish(ctx)?;
            // let mut canvas = Canvas::from_screen_image(
            //     ctx,
            //     &mut self.foreground,
            //     Color {
            //         r: 0.0,
            //         g: 0.0,
            //         b: 0.0,
            //         a: 0.0,
            //     },
            // );
            // self.draw_foreground(ctx, &mut canvas, camera)?;
            // canvas.finish(ctx)?;

            self.inited = true;
        }

        let mut canvas = Canvas::from_screen_image(
            ctx,
            &mut self.foreground,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        );
        self.draw_foreground(ctx, &mut canvas, camera)?;
        if self.draw_effect_layer && self.has_ani {
            let mut canvas = Canvas::from_screen_image(ctx, &mut self.effect_layer, Color::BLACK);
            self.draw_effect_layer("animationlayer", &mut canvas, camera);
            canvas.finish(ctx)?;
        }
        // canvas.finish(ctx)?;
        //let mut canvas = Canvas::from_screen_image(ctx, &mut self.sprite_layer, Color::BLACK);
        canvas.set_blend_mode(BlendMode::ADD);
        let map_coord = self.world_to_map(actor.position().x, actor.position().y);
        // println!("map_coord:{:?}", map_coord);

        let (x, y) = (map_coord[0] as usize, map_coord[1] as usize);
        self.map
            .layers
            .get("objectlayer")
            .and_then(|layer: &tiled::Layer| layer.get_tile_xy(x, y))
            .and_then(|tile| self.get_texture(&tile.id))
            .and_then(|tex| {
                let world_position = [
                    x as f32 * self.map.tile_width as f32 + self.tile_offset_x as f32,
                    y as f32 * self.map.tile_height as f32 + self.tile_offset_y as f32,
                ];
                // println!(
                //     "actor.position: {:?} object.position:{:?} {}x{}",
                //     actor.position(),
                //     world_position,
                //     tex.image.width(),
                //     tex.image.height()
                // );
                if actor.position().y + 85. < world_position[1] + tex.image.height() as f32 {
                    canvas.set_blend_mode(BlendMode::ADD);
                } else {
                    canvas.set_blend_mode(BlendMode::ALPHA);
                }
                None::<()>
            });
        actor.draw(&mut canvas, camera.clone());
        canvas.finish(ctx)?;

        Ok(())
    }

    pub fn draw_background(&mut self, canvas: &mut Canvas, camera: &Camera2D) -> GameResult {
        let camera = &camera.clone().with_scale([1., 1.]);
        if self.draw_tile_layer {
            self.draw_tile_layer("tilelayer", canvas, camera);
        }

        if self.draw_smtile_layer {
            self.draw_smtile_layer("smtilelayer", canvas, camera);
        }
        Ok(())
    }

    pub fn draw_foreground(
        &mut self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        camera: &Camera2D,
    ) -> GameResult {
        let camera = &camera.clone().with_scale([1., 1.]);

        if self.draw_object_layer {
            self.draw_object_layer("objectlayer", canvas, camera);
            self.draw_object_layer("doorlayer", canvas, camera);
        }

        if self.draw_collision_layer {
            self.draw_collision_layer(ctx, "barrierlayer", canvas, camera);
            self.draw_collision_layer(ctx, "fnzlayer", canvas, camera);
        }
        if self.draw_grid_layer {
            self.draw_grid_layer(ctx, canvas, camera);
        }

        Ok(())
    }
    pub fn draw_collision_layer(
        &self,
        ctx: &mut Context,
        layer: &str,
        canvas: &mut Canvas,
        camera: &Camera2D,
    ) {
        let mut batch = InstanceArray::new(ctx, None);
        if let Some(layer) = self.map.layers.get(layer) {
            let top = self.top as usize;
            let left = self.left as usize;
            let right = self.right as usize;
            let bottom = self.bottom as usize;
            for y in top..=bottom {
                for x in left..=right {
                    layer.get_tile_xy(x, y).is_some().then(|| {
                        let world_position = [
                            x as f32 * self.map.tile_width as f32 + self.object_offset_x as f32,
                            y as f32 * self.map.tile_height as f32 + self.object_offset_y as f32,
                        ];
                        let screen_position = camera.world_to_screen(world_position);
                        batch.push(
                            DrawParam::default()
                                .scale([1., -1.])
                                .dest(screen_position)
                                .color(Color::RED),
                        );
                    });
                }
            }
            canvas.draw_instanced_mesh(self.mesh.clone(), &batch, DrawParam::default());
        }
    }

    pub fn draw_grid_layer(&self, ctx: &mut Context, canvas: &mut Canvas, camera: &Camera2D) {
        let mut batch = InstanceArray::new(ctx, None);

        let top = self.top as usize;
        let left = self.left as usize;
        let right = self.right as usize;
        let bottom = self.bottom as usize;
        for y in top..=bottom {
            for x in left..=right {
                let world_position = [
                    x as f32 * self.map.tile_width as f32,
                    y as f32 * self.map.tile_height as f32,
                ];
                let screen_position = camera.world_to_screen(world_position);
                batch.push(
                    DrawParam::default()
                        .scale([1., -1.])
                        .dest(screen_position)
                        .color(Color::WHITE),
                );
            }
        }
        canvas.draw_instanced_mesh(self.mesh.clone(), &batch, DrawParam::default());
    }

    pub fn draw_tile_layer(&self, layer: &str, canvas: &mut Canvas, camera: &Camera2D) {
        if let Some(layer) = self.map.layers.get(layer) {
            let top = self.top as usize;
            let left = self.left as usize;
            let right = self.right as usize;
            let bottom = self.bottom as usize;
            for y in top..=bottom {
                for x in left..=right {
                    if x % 2 == 0 && y % 2 == 0 {
                        layer
                            .get_tile_xy(x, y)
                            .and_then(|tile| self.get_texture(&tile.id))
                            .and_then(|texture| {
                                let world_position = [
                                    x as f32 * self.map.tile_width as f32
                                        + self.tile_offset_x as f32,
                                    y as f32 * self.map.tile_height as f32
                                        + self.tile_offset_y as f32,
                                ];
                                let screen_position = camera.world_to_screen(world_position);
                                canvas.draw(texture, DrawParam::default().dest(screen_position));
                                None::<()>
                            });
                    }
                }
            }
        }
    }

    pub fn draw_smtile_layer(&self, layer: &str, canvas: &mut Canvas, camera: &Camera2D) {
        if let Some(layer) = self.map.layers.get(layer) {
            let top = self.top as usize;
            let left = self.left as usize;
            let right = self.right as usize;
            let bottom = self.bottom as usize;
            for y in top..=bottom {
                for x in left..=right {
                    layer
                        .get_tile_xy(x, y)
                        .and_then(|tile| self.get_texture(&tile.id))
                        .and_then(|texture| {
                            let world_position = [
                                x as f32 * self.map.tile_width as f32,
                                y as f32 * self.map.tile_height as f32,
                            ];
                            let screen_position = camera.world_to_screen(world_position);
                            canvas.draw(texture, DrawParam::default().dest(screen_position));
                            None::<()>
                        });
                }
            }
        }
    }

    pub fn draw_object_layer(&self, layer: &str, canvas: &mut Canvas, camera: &Camera2D) {
        if let Some(layer) = self.map.layers.get(layer) {
            let top = self.top as usize;
            let left = self.left as usize;
            let right = self.right as usize;
            let bottom = self.bottom as usize;
            for y in top..=bottom {
                for x in left..=right {
                    layer
                        .get_tile_xy(x, y)
                        .and_then(|tile| {
                            // println!("draw object layer:({x},{y}) tid:{:?}", tile.id);
                            self.get_texture(&tile.id)
                        })
                        .and_then(|tex| {
                            let world_position = [
                                x as f32 * self.map.tile_width as f32 + self.object_offset_x as f32,
                                y as f32 * self.map.tile_height as f32
                                    + self.object_offset_y as f32,
                            ];
                            let screen_position = camera.world_to_screen(world_position);
                            canvas.set_blend_mode(BlendMode::ALPHA);
                            canvas.draw(tex, screen_position);
                            None::<()>
                        });
                }
            }
        }
    }

    pub fn draw_effect_layer(&self, layer: &str, canvas: &mut Canvas, camera: &Camera2D) {
        if let Some(_layer) = self.map.layers.get(layer) {
            let top = self.top as usize;
            let left = self.left as usize;
            let right = self.right as usize;
            let bottom = self.bottom as usize;
            for y in top..=bottom {
                for x in left..=right {
                    self.animations
                        .get(&(x, y))
                        .and_then(|ani| {
                            // println!(
                            //     "draw effect layer:({x},{y}) frames:{} {}",
                            //     ani.sequence.len(),
                            //     ani.fps
                            // );
                            ani.next_frame_aid()
                        })
                        .and_then(|aid| {
                            println!(
                                "draw effect layer:({x},{y}) {aid:?} {}",
                                self.get_texture(&(aid.name(), aid.idx()).into()).is_some()
                            );
                            self.get_texture(&(aid.name(), aid.idx()).into())
                        })
                        .and_then(|tex| {
                            println!(
                                "draw effect layer:({x},{y}) offx:{} offy:{}",
                                tex.offset_x, tex.offset_y
                            );
                            let world_position = [
                                x as f32 * self.map.tile_width as f32
                                + tex.offset_x as f32//大地图要偏移?
                                    + self.object_offset_x as f32,
                                y as f32 * self.map.tile_height as f32
                                     + tex.offset_y as f32//大地图要偏移?
                                    + self.object_offset_y as f32,
                            ];
                            let screen_position = camera.world_to_screen(world_position);
                            //动画混合绘制
                            canvas.set_blend_mode(BlendMode::ADD);
                            canvas.draw(tex, DrawParam::default().dest(screen_position));

                            None::<()>
                        });
                }
            }
        }
    }

    pub fn draw_minimap(&mut self, ctx: &mut Context, canvas: &mut Canvas) {
        if let Some(mmap) = self.minimap.as_ref() {
            let (w, h) = ctx.gfx.drawable_size();
            //计算缩放比例
            let scale = [
                (w / 5.) / mmap.image.width() as f32,
                (h / 5.) / mmap.image.height() as f32,
            ];
            //构造缩放矩阵
            let mat4 = Mat4::from_scale(vec3(scale[0], scale[1], 1.));
            //计算图片大小
            let size = mat4.transform_point3(
                [mmap.image.width() as f32, mmap.image.height() as f32, 0.].into(),
            );
            let dx = w - size.x;
            let dy = size.y;
            canvas.draw(
                mmap,
                DrawParam::new()
                    .dest([dx, dy])
                    .scale(scale)
                    .color(Color::from_rgba(255, 255, 255, 180)),
            );
        }
    }
}

impl LogicHandler for Atlas {
    fn update(&mut self, ctx: &mut Context, camera: &mut Camera2D) -> GameResult {
        let camera = &camera.clone().with_scale([1., 1.]);
        self.update_data(camera);
        let secs = ctx.time.delta().as_secs_f64();
        for ani in self.animations.values_mut() {
            ani.advance_and_maybe_wrap(secs);
        }
        Ok(())
    }
}

// impl Drawable for Atlas {
//     fn dimensions(
//         &self,
//         _gfx: &impl ggez::context::Has<graphics::GraphicsContext>,
//     ) -> Option<graphics::Rect> {
//         None
//     }
//     fn draw(&self, canvas: &mut graphics::Canvas, param: impl Into<DrawParam>) {
//         let param: DrawParam = param.into();
//         canvas.draw(&self.bg_image, param.clone());
//         canvas.set_blend_mode(BlendMode::ALPHA);
//         canvas.draw(&self.fg_image, param);
//     }
// }

fn read_mmap(name: &str) -> Option<usize> {
    std::fs::read_to_string(format!("{}/minimap.txt", Map::MAP_DIR).as_str())
        .ok()
        .and_then(|lines| {
            for line in lines.lines() {
                if line.starts_with(name) {
                    return line.replace(name, "").trim().parse().ok();
                }
            }
            None
        })
}
