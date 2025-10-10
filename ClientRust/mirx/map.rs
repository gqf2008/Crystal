use crate::tiled::{self, TileId, Value};
use ggegui::egui::vec2;
use ggez::{
    context::HasMut,
    glam::Vec2,
    graphics::{
        self, BlendComponent, BlendFactor, BlendMode, BlendOperation, Color, DrawMode, DrawParam,
        Image, InstanceArray, Mesh, MeshBuilder, Quad, Rect, Transform,
    },
    mint::Point2,
    Context,
};
use std::{collections::HashMap, io::Write};

#[derive(Debug)]
pub struct Texture {
    pub offset_x: i16,
    pub offset_y: i16,
    pub opacity: u8,
    pub image: Image,
}

//世界坐标系
//(0,0)-------->(x,0)
// |
// |
// |
// |
//(0,y)-------->(x,y)
// 定义 Map 结构体
pub struct Map {
    // 地图宽度，实际是指世界宽度
    pub mw: i32,
    // 地图高度，实际是指世界高度
    pub mh: i32,
    // 角色身处横坐标
    pub x: i16,
    // 角色身处纵坐标
    pub y: i16,
    // 地图绘制区域左上角(相对于游戏区域直角坐标系)
    pub px: i16,
    // 地图绘制区域右上角(相对于游戏区域直角坐标系)
    pub py: i16,
    // 地图绘制区域宽度
    pub gw: i16,
    // 地图绘制区域高度
    pub gh: i16,
    // 绘图区域左上角为地图块第几列
    pub tws: i16,
    // 绘图区域左上角为地图块第几行
    pub ths: i16,
    // 绘制区域右下角为地图块第几列
    pub twe: i16,
    // 绘制区域右下角为地图块第几行
    pub the: i16,
    // 纹理图片需要准备的坐标左上角列数
    pub pws: i16,
    // 纹理图片需要准备的左上角行数
    pub phs: i16,
    // 纹理图片需要准备的右下角列数
    pub pwe: i16,
    // 纹理图片需要准备的右下角行数
    pub phe: i16,
    // 地图名称
    pub name: String,
    // 地图信息
    pub map: tiled::Map,

    pub textures: HashMap<TileId, Texture>,

    pub tile_offset_x: i16,
    pub tile_offset_y: i16,
    pub object_offset_x: i16,
    pub object_offset_y: i16,

    pub draw_tile_layer: bool,
    pub draw_smtile_layer: bool,
    pub draw_object_layer: bool,
    pub draw_effect_layer: bool,
    pub draw_collision_layer: bool,
    pub draw_mesh_layer: bool,

    pub mesh: graphics::Mesh,
    //pub blend_mode: BlendMode,
}

impl Map {
    // 地图磁块宽
    pub const PIXEL_WIDTH_PER_TILE: u8 = 48;
    // 地图磁块高
    pub const PIXEL_HEIGHT_PER_TILE: u8 = 32;
    // 绘制地图时向左延伸块儿数量
    pub const EXTEND_LEFT: u8 = 5;
    // 绘制地图时向右延伸块儿数量
    pub const EXTEND_RIGHT: u8 = 5;
    // 绘制地图时向下延伸块儿数量
    pub const EXTEND_BOTTOM: u8 = 25;
    pub fn new(name: &str, map: tiled::Map, mesh: Mesh) -> Self {
        let mw = map.width as i32 * Self::PIXEL_WIDTH_PER_TILE as i32; // 地图宽度(像素)
        let mh = map.height as i32 * Self::PIXEL_HEIGHT_PER_TILE as i32; // 地图高度(像素)
        let map = map.rotate_90().fliph(); //变换到世界坐标系
        Map {
            mw,
            mh,
            x: 0,
            y: 0,
            px: 0,
            py: 0,
            gw: 0,
            gh: 0,
            tws: 0,
            ths: 0,
            twe: 0,
            the: 0,
            pws: 0,
            phs: 0,
            pwe: 0,
            phe: 0,
            name: name.to_string(),
            map,

            tile_offset_x: 0,
            tile_offset_y: 0,
            object_offset_x: 0,
            object_offset_y: -32,
            textures: HashMap::new(),
            draw_tile_layer: true,
            draw_smtile_layer: true,
            draw_object_layer: true,
            draw_effect_layer: true,
            draw_collision_layer: false,
            draw_mesh_layer: false,
            mesh,
            //blend_mode: BlendMode::REPLACE,
        }
    }

    // 获取角色身处横坐标
    pub fn role_x(&self) -> i16 {
        self.x
    }

    // 获取角色身处纵坐标
    pub fn role_y(&self) -> i16 {
        self.y
    }

    #[inline]
    //移动到地图坐标位置， 移动角色身处的坐标(相对于地图)
    pub fn move_to(&mut self, x: i32, y: i32) -> &mut Self {
        self.x = x as i16;
        self.y = y as i16;
        if self.x < 0 {
            self.x = 0;
        }
        if self.x > self.map.width as i16 {
            self.x = self.map.width as i16;
        }
        if self.y < 0 {
            self.y = 0;
        }
        if self.y > self.map.height as i16 {
            self.y = self.map.height as i16;
        }
        self
    }

    //移动到地图坐标位置， 移动角色身处的坐标(相对于地图)
    pub fn move_by(&mut self, x: i32, y: i32) -> &mut Self {
        let x = self.x as i32 + x;
        let y = self.y as i32 + y;
        self.move_to(x, y)
    }

    #[inline]
    pub fn move_down(&mut self, step: i32) -> &mut Self {
        let y = self.y as i32 + step;
        let y = y.min(self.map.height as i32);
        self.move_to(self.x as i32, y)
    }
    #[inline]
    pub fn move_up(&mut self, step: i32) -> &mut Self {
        let y = self.y as i32 - step;
        let y = if y < 0 { 0 } else { y };
        self.move_to(self.x as i32, y)
    }
    #[inline]
    pub fn move_left(&mut self, step: i32) -> &mut Self {
        let x = self.x as i32 - step;
        let x = if x < 0 { 0 } else { x };
        self.move_to(x, self.y as i32)
    }
    #[inline]
    pub fn move_right(&mut self, step: i32) -> &mut Self {
        let x = self.x as i32 + step;
        let x = x.min(self.map.width as i32);
        self.move_to(x, self.y as i32)
    }

    pub fn update_data(&mut self, ctx: &mut ggez::Context) {
        let (vw, vh) = ctx.gfx.drawable_size();
        let vw = vw as i32;
        let vh = vh as i32;
        self.px = if vw > self.mw {
            ((vw - self.mw) / 2) as i16
        } else {
            0
        };
        self.py = if vh > self.mh {
            ((vh - self.mh) / 2) as i16
        } else {
            0
        };

        self.gw = if vw > self.mw {
            self.mw as i16
        } else {
            vw as i16
        };

        self.gh = if vh > self.mh {
            self.mh as i16
        } else {
            vh as i16
        };

        self.tws = (self.x - (self.gw / Self::PIXEL_WIDTH_PER_TILE as i16 - 1) / 2) as i16;
        if self.tws < 0 {
            self.tws = 0;
        }

        self.ths = (self.y - (self.gh / Self::PIXEL_HEIGHT_PER_TILE as i16 - 1) / 2) as i16;
        if self.ths < 0 {
            self.ths = 0;
        }

        self.twe =
            self.tws + self.gw / Self::PIXEL_WIDTH_PER_TILE as i16 + Self::EXTEND_RIGHT as i16;
        if self.twe > self.map.width as i16 {
            self.twe = self.map.width as i16;
        }

        self.the =
            self.ths + self.gh / Self::PIXEL_HEIGHT_PER_TILE as i16 + Self::EXTEND_BOTTOM as i16;
        if self.the > self.map.height as i16 {
            self.the = self.map.height as i16;
        }

        self.pws = self.x - (self.gw / Self::PIXEL_WIDTH_PER_TILE as i16 - 1);
        if self.pws < 0 {
            self.pws = 0;
        }

        self.phs = self.y - (self.gh / Self::PIXEL_HEIGHT_PER_TILE as i16 - 1);
        if self.phs < 0 {
            self.phs = 0;
        }

        self.pwe = self.tws + self.gw / Self::PIXEL_WIDTH_PER_TILE as i16 * 2;
        if self.pwe > self.map.width as i16 {
            self.pwe = self.map.width as i16;
        }

        self.phe = self.ths + self.gh / Self::PIXEL_HEIGHT_PER_TILE as i16 * 2;
        if self.phe > self.map.height as i16 {
            self.phe = self.map.height as i16;
        }

        if (self.gw / Self::PIXEL_WIDTH_PER_TILE as i16 - 1) % 2 != 0 {
            self.twe -= 1;
        }
        if (self.gh / Self::PIXEL_HEIGHT_PER_TILE as i16 - 1) % 2 != 0 {
            self.the -= 1;
        }
    }
    pub fn load_all_texture(mut self, ctx: &mut ggez::Context) -> Self {
        for (tid, chunk) in self.map.tilesets.iter_mut() {
            self.textures
                .insert(tid.clone(), chunk.to_texture(ctx.retrieve_mut()));
        }
        self.map.tilesets.clear();
        self
    }
    pub fn get_texture(&self, tid: &TileId) -> Option<&Texture> {
        self.textures.get(tid)
    }

    pub fn draw_layer(
        &mut self,
        ctx: &mut Context,
        canvas: &mut ggez::graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        let param = param.into();
        if self.draw_tile_layer {
            self.draw_tile_layer(ctx, "tilelayer", canvas, param.clone());
        }
        if self.draw_smtile_layer {
            self.draw_smtile_layer(ctx, "smtilelayer", canvas, param.clone());
        }

        if self.draw_object_layer {
            self.draw_object_layer(ctx, "objectlayer", canvas, param.clone());
            self.draw_object_layer(ctx, "doorlayer", canvas, param.clone());
        }

        if self.draw_effect_layer {
            self.draw_effect_layer(ctx, "animationlayer", canvas, param.clone());
        }

        if self.draw_collision_layer {
            self.draw_mesh_layer(ctx, "barrierlayer", canvas, param.clone().color(Color::RED));
            self.draw_mesh_layer(ctx, "fnzlayer", canvas, param.clone().color(Color::BLUE));
        }
    }

    pub fn draw_mesh_layer(
        &self,
        ctx: &mut Context,
        layer: &str,
        canvas: &mut ggez::graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        let mut batch = InstanceArray::new(ctx, None);

        if let Some(layer) = self.map.layers.get(layer) {
            let param: DrawParam = param.into();
            let mut left = self.tws as i32 - Self::EXTEND_LEFT as i32;
            if left < 0 {
                left = 0;
            }
            for w in left..self.twe as i32 {
                for h in self.ths as i32..self.the as i32 + 1 {
                    layer
                        .get_tile_xy(w as usize, h as usize)
                        .is_some()
                        .then(|| {
                            let cpx = self.px as i32
                                + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32
                                + self.tile_offset_x as i32;
                            let cpy = self.py as i32
                                + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32
                                + self.tile_offset_y as i32;
                            let param = param.clone().dest(Vec2::new(cpx as f32, cpy as f32));
                            batch.push(param);
                        });
                }
            }
            canvas.draw_instanced_mesh(self.mesh.clone(), &batch, DrawParam::default());
        }
    }

    pub fn dump_tile_layer(&self, layer: &str) {
        if let Some(layer) = self.map.layers.get(layer) {
            let mut lines = vec![];
            for w in 0..self.map.width as i32 {
                for h in 0..self.map.height as i32 {
                    let cpx = self.px as i32
                        + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32
                        + self.tile_offset_x as i32;
                    let cpy = self.py as i32
                        + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32
                        + self.tile_offset_y as i32;
                    if let Some(tile) = layer.get_tile_xy(w as usize, h as usize) {
                        lines.push(format!("({w},{h}).({cpx},{cpy}).{:?}", tile.id));
                    } else {
                        lines.push(format!("({w},{h}).({cpx},{cpy}).None"));
                    }
                }
            }
            let content = lines.join("\n");
            std::fs::write(format!("{}.{}.map", self.name, layer.name), content).unwrap();
        }
    }

    pub fn draw_tile_layer(
        &mut self,
        ctx: &mut Context,
        layer: &str,
        canvas: &mut ggez::graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        // let mut instances = HashMap::new();
        if let Some(layer) = self.map.layers.get(layer) {
            let param: DrawParam = param.into();

            let mut left = self.tws as i32 - Self::EXTEND_LEFT as i32;
            if left < 0 {
                left = 0;
            }
            for w in left..self.twe as i32 {
                for h in self.ths as i32..self.the as i32 + 1 {
                    layer.get_tile_xy(w as usize, h as usize).and_then(|tile| {
                        let cpx = self.px as i32
                            + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32
                            + self.tile_offset_x as i32;
                        let cpy = self.py as i32
                            + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32
                            + self.tile_offset_y as i32;
                        if self.get_texture(&tile.id).is_none() {
                            println!("({w},{h}).({cpx},{cpy}).None");
                        }
                        self.get_texture(&tile.id).and_then(|texture| {
                            if w % 2 != 0 || h % 2 != 0 {
                                canvas.draw(
                                    &texture.image,
                                    param.dest(Vec2::new(cpx as f32, cpy as f32)),
                                );
                                // let instance = instances
                                //     .entry(&tile.id)
                                //     .or_insert(InstanceArray::new(ctx, texture.image.clone()));
                                // instance.push(param.dest(Vec2::new(cpx as f32, cpy as f32)));
                            }

                            None::<()>
                        });
                        None::<()>
                    });
                }
            }
            // for instance in instances.values() {
            //     canvas.draw(instance, DrawParam::default());
            // }
            // instances.clear();
            for w in left..self.twe as i32 {
                for h in self.ths as i32..self.the as i32 + 1 {
                    layer.get_tile_xy(w as usize, h as usize).and_then(|tile| {
                        let cpx = self.px as i32
                            + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32
                            + self.tile_offset_x as i32;
                        let cpy = self.py as i32
                            + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32
                            + self.tile_offset_y as i32;
                        if self.get_texture(&tile.id).is_none() {
                            println!("({w},{h}).({cpx},{cpy}).None");
                        }
                        self.get_texture(&tile.id).and_then(|texture| {
                            if w % 2 == 0 && h % 2 == 0 {
                                canvas.set_blend_mode(BlendMode::REPLACE);
                                canvas.draw(
                                    &texture.image,
                                    param.dest(Vec2::new(cpx as f32, cpy as f32)),
                                );
                                // let instance = instances
                                //     .entry(&tile.id)
                                //     .or_insert(InstanceArray::new(ctx, texture.image.clone()));
                                // instance.push(param.dest(Vec2::new(cpx as f32, cpy as f32)));
                            }
                            None::<()>
                        });
                        None::<()>
                    });
                }
            }

            // for instance in instances.values() {
            //     canvas.set_blend_mode(BlendMode::REPLACE);
            //     canvas.draw(instance, DrawParam::default());
            // }
        }
    }

    pub fn draw_smtile_layer(
        &mut self,
        ctx: &mut Context,
        layer: &str,
        canvas: &mut ggez::graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        if let Some(layer) = self.map.layers.get(layer) {
            let param: DrawParam = param.into();
            //let mut instances = HashMap::new();
            let mut left = self.tws as i32 - Self::EXTEND_LEFT as i32;
            if left < 0 {
                left = 0;
            }
            for w in left..self.twe as i32 {
                for h in self.ths as i32..self.the as i32 + 1 {
                    layer.get_tile_xy(w as usize, h as usize).and_then(|tile| {
                        let cpx = self.px as i32
                            + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32
                            + self.tile_offset_x as i32;
                        let cpy = self.py as i32
                            + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32
                            + self.tile_offset_y as i32;
                        if self.get_texture(&tile.id).is_none() {
                            println!("({w},{h}).({cpx},{cpy}).None");
                        }
                        self.get_texture(&tile.id).and_then(|texture| {
                            //canvas.set_blend_mode(BlendMode::REPLACE);
                            canvas.draw(
                                &texture.image,
                                param.clone().dest(Vec2::new(cpx as f32, cpy as f32)),
                            );
                            // let instance = instances
                            //     .entry(&tile.id)
                            //     .or_insert(InstanceArray::new(ctx, texture.image.clone()));
                            // instance.push(param.clone().dest(Vec2::new(cpx as f32, cpy as f32)));

                            None::<()>
                        });
                        None::<()>
                    });
                }
            }
            // for instance in instances.values() {
            //     canvas.draw(instance, DrawParam::default());
            // }
        }
    }

    pub fn draw_object_layer(
        &mut self,
        ctx: &mut Context,
        layer: &str,
        canvas: &mut ggez::graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        if let Some(layer) = self.map.layers.get(layer) {
            //  let mut instances = HashMap::new();
            let param: DrawParam = param.into();
            let mut left = self.tws as i32 - Self::EXTEND_LEFT as i32;
            if left < 0 {
                left = 0;
            }
            for w in left..self.twe as i32 {
                for h in self.ths as i32..self.the as i32 + 1 {
                    layer.get_tile_xy(w as usize, h as usize).and_then(|tile| {
                        self.get_texture(&tile.id).and_then(|texture| {
                            let cpx = self.px as i32
                                + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32
                                + self.object_offset_x as i32;
                            let cpy = self.py as i32
                                + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32
                                + self.object_offset_y as i32;
                            // let (cpx, cpy) =
                            //     if let Transform::Values { scale, .. } = param.transform {
                            //         (cpx as f32 * scale.x, cpy as f32 * scale.y)
                            //     } else {
                            //         (cpx as f32, cpy as f32)
                            //     };

                            canvas.set_blend_mode(BlendMode::ALPHA);
                            let param = param.clone().dest(Vec2::new(cpx as f32, cpy as f32));
                            canvas.draw(&texture.image, param);
                            // let instance = instances
                            //     .entry(&tile.id)
                            //     .or_insert(InstanceArray::new(ctx, texture.image.clone()));
                            // instance.push(param.clone().dest(Vec2::new(cpx as f32, cpy as f32)));
                            None::<()>
                        });
                        None::<()>
                    });
                }
            }
            // for instance in instances.values() {
            //     canvas.set_blend_mode(BlendMode::ALPHA);
            //     canvas.draw(instance, DrawParam::default());
            // }
        }
    }

    pub fn draw_effect_layer(
        &mut self,
        ctx: &mut Context,
        layer: &str,
        canvas: &mut ggez::graphics::Canvas,
        param: impl Into<DrawParam>,
    ) {
        if let Some(layer) = self.map.layers.get(layer) {
            let param: DrawParam = param.into();
            let mut left = self.tws as i32 - Self::EXTEND_LEFT as i32;
            if left < 0 {
                left = 0;
            }
            // println!(
            //     "draw animation layer: {}, left: {}, right: {} top: {} bottom: {}",
            //     layer.name,
            //     left,
            //     self.twe,
            //     self.ths,
            //     self.the as i32 + 1
            // );
            for w in left..self.twe as i32 {
                for h in self.ths as i32..self.the as i32 + 1 {
                    layer.get_tile_xy(w as usize, h as usize).and_then(|tile| {
                        let cpx = self.px as i32
                            + (w - self.tws as i32) * Self::PIXEL_WIDTH_PER_TILE as i32;
                        let cpy = self.py as i32
                            + (h - self.ths as i32) * Self::PIXEL_HEIGHT_PER_TILE as i32;
                        let frames = tile
                            .properties
                            .get("frames")
                            .unwrap_or(&Value::U8(0))
                            .as_i16();

                        let fps = tile.properties.get("fps").unwrap_or(&Value::U8(0)).as_i16();
                        // println!("frames:{frames} fps:{fps}");
                        let mut ati = (3 - 1) / (60 / frames);
                        if ati < 0 {
                            ati = 0
                        }
                        if ati >= frames {
                            ati = frames - 1;
                        }
                        for idx in tile.id.idx()..tile.id.idx() + ati {
                            self.get_texture(&(&tile.id.tileset(), idx).into())
                                .and_then(|texture| {
                                    let cpx = cpx + texture.offset_x as i32;
                                    let cpy = cpy + texture.offset_y as i32;
                                    // let (cpx, cpy) =
                                    //     if let Transform::Values { scale, .. } = param.transform {
                                    //         (cpx as f32 * scale.x, cpy as f32 * scale.y)
                                    //     } else {
                                    //         (cpx as f32, cpy as f32)
                                    //     };

                                    //动画混合绘制
                                    canvas.set_blend_mode(BlendMode::ADD);
                                    let param =
                                        param.clone().dest(Vec2::new(cpx as f32, cpy as f32));
                                    canvas.draw(&texture.image, param);

                                    None::<()>
                                });
                        }
                        None::<()>
                    });
                }
            }
        }
    }
}
