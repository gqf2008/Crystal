use crate::{
    map::Texture,
    matrix::Transform,
    palette::{Color, PALETTE},
};
use bon::{builder, Builder};
use byteorder::{ReadBytesExt, LE};
use flate2::write::ZlibDecoder;
use std::{
    collections::HashMap,
    io::{BufReader, Read, Seek, Write},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileId(String, i16);

impl TileId {
    pub fn tileset(&self) -> &str {
        self.0.as_str()
    }

    pub fn idx(&self) -> i16 {
        self.1
    }
}

// impl From<(&str, i16)> for TileId {
//     fn from(value: (&str, i16)) -> Self {
//         TileId(value.0.to_string(), value.1)
//     }
// }
impl<S: ToString> From<(S, i16)> for TileId {
    fn from(value: (S, i16)) -> Self {
        TileId(value.0.to_string(), value.1)
    }
}

impl<S: ToString> From<(S, i32)> for TileId {
    fn from(value: (S, i32)) -> Self {
        TileId(value.0.to_string(), value.1 as i16)
    }
}

impl<S: ToString> From<(S, usize)> for TileId {
    fn from(value: (S, usize)) -> Self {
        TileId(value.0.to_string(), value.1 as i16)
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(on(String, into))]
pub struct Tiled {
    #[builder(into)]
    pub id: TileId,
    //pub id:i16,//地图文件中的索引号
    //是否水平翻转
    #[builder(default)]
    pub flip_h: bool,
    //是否垂直翻转
    #[builder(default)]
    pub flip_v: bool,
    //是否对角线翻转
    #[builder(default)]
    pub flip_d: bool,
    //属性值
    #[builder(default)]
    pub properties: HashMap<String, Value>,
}

#[derive(Debug, Clone, Builder)]
#[builder(on(String, into))]
pub struct Layer {
    pub name: String,
    pub width: u32,  //单位:Tiled
    pub height: u32, //单位:Tiled
    #[builder(default)]
    pub tile_width: i32,
    #[builder(default)]
    pub tile_height: i32,
    #[builder(default = 1)]
    pub opacity: u8,
    #[builder(default = true)]
    pub visible: bool,
    #[builder(default)]
    pub x: i32,
    #[builder(default)]
    pub y: i32,
    #[builder(default)]
    pub tiles: Vec<Option<Tiled>>,
}

impl Layer {
    pub fn rotate_90(&mut self) -> &mut Self {
        self.tiles = self
            .tiles
            .rotate_90(self.width as usize, self.height as usize);
        self
    }

    pub fn rotate_180(&mut self) -> &mut Self {
        self.tiles = self
            .tiles
            .rotate_180(self.width as usize, self.height as usize);
        self
    }
    pub fn rotate_270(&mut self) -> &mut Self {
        self.tiles = self
            .tiles
            .rotate_270(self.height as usize, self.width as usize);
        self
    }
    pub fn flipv(&mut self) -> &mut Self {
        self.tiles.flipv_in_place(self.width as usize);
        self
    }

    pub fn fliph(&mut self) -> &mut Self {
        self.tiles.fliph_in_place(self.width as usize);
        self
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn add_tile(&mut self, tile: Option<Tiled>) -> &mut Self {
        self.tiles.push(tile);
        self
    }

    pub fn get_tile(&self, idx: usize) -> Option<&Tiled> {
        self.tiles.get(idx).and_then(|tile| tile.as_ref())
    }

    pub fn get_tile_xy(&self, x: usize, y: usize) -> Option<&Tiled> {
        let idx = x + y * self.width as usize;
        self.get_tile(idx)
    }
}

#[derive(Debug, Clone, Builder)]
#[builder(on(String, into))]
pub struct TileSet {
    pub name: String,
    #[builder(default)]
    pub tilecount: u32,
    // pub texture: Vec<u8>,
    #[builder(default)]
    pub tile_width: i32,
    #[builder(default)]
    pub tile_height: i32,
    #[builder(default)]
    pub columns: u32,
    #[builder(default)]
    pub spacing: i32,
    #[builder(default)]
    pub margin: i32,
    #[builder(default)]
    pub chunks: HashMap<i16, Chunk>, //图块
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Orientation {
    #[default]
    Orthogonal,
    Isometric,
    Staggered,
    Hexagonal,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum RenderOrder {
    RightDown,
    LeftDown,
    #[default]
    RightUp,
    LeftUp,
}

//地图坐标系
//(0,0)-------->(0,y)
// |
// |
// |
// |
//(x,0)-------->(x,y)
#[derive(Builder)]
pub struct Map {
    pub width: u32,
    pub height: u32,
    #[builder(default)]
    pub orientation: Orientation,
    #[builder(default)]
    pub renderorder: RenderOrder,
    #[builder(default = 48)]
    pub tile_width: u32,
    #[builder(default = 32)]
    pub tile_height: u32,
    #[builder(default)]
    pub layers: HashMap<String, Layer>, //大图层只有三层，分别是tile、small_tile、object层
    #[builder(default)]
    pub tilesets: HashMap<TileId, Chunk>, //图像集
    pub tile_format: TileFormat,
    // #[builder(default=Assets::new())]
    // assets: Assets,
}
impl Map {
    pub fn get_chunk(&self, tid: &TileId) -> Option<&Chunk> {
        self.tilesets.get(tid)
    }

    pub fn layer_of_chunks(&self, layer: &str) -> usize {
        self.layers.get(layer).map_or(0, |layer| layer.tile_count())
    }

    pub fn rotate_90(mut self) -> Self {
        for layer in self.layers.values_mut() {
            layer.rotate_90();
        }
        self
    }

    pub fn rotate_180(mut self) -> Self {
        for layer in self.layers.values_mut() {
            layer.rotate_180();
        }
        self
    }
    pub fn rotate_270(mut self) -> Self {
        for layer in self.layers.values_mut() {
            layer.rotate_270();
        }
        self
    }
    pub fn flipv(mut self) -> Self {
        for layer in self.layers.values_mut() {
            layer.flipv();
        }
        self
    }

    pub fn fliph(mut self) -> Self {
        for layer in self.layers.values_mut() {
            layer.fliph();
        }
        self
    }

    pub fn load_tileset(mut self) -> anyhow::Result<Self> {
        let mut assets = Assets::new();
        for layer in self.layers.values() {
            for tile in layer.tiles.iter().filter_map(|item| item.as_ref()) {
                if tile.id.tileset().is_empty() {
                    continue;
                }
                if tile.id.tileset() == "animationlayer" {
                    //加载动画帧
                    let frames = tile
                        .properties
                        .get("frames")
                        .unwrap_or(&Value::U8(0))
                        .as_i16();
                    for idx in tile.id.idx()..tile.id.idx() + frames {
                        let tid = (tile.id.tileset(), idx).into();
                        assets.load_image(&tid)?.and_then(|chunk| {
                            self.tilesets.entry(tid).or_insert(chunk);
                            None::<()>
                        });
                    }
                    continue;
                }
                assets.load_image(&tile.id)?.and_then(|chunk| {
                    self.tilesets.entry(tile.id.clone()).or_insert(chunk);
                    None::<()>
                });
            }
        }
        Ok(self)
    }
}

impl Map {
    pub const MAP_DIR: &'static str = "map";
    pub fn load(name: &str) -> anyhow::Result<Self> {
        let reader = std::fs::read(name)?;
        let mut reader = std::io::Cursor::new(reader);
        let width = reader.read_u16::<LE>()? as u32;
        let height = reader.read_u16::<LE>()? as u32;
        // let mut buf = [0u8; 17];
        // reader.read_exact(&mut buf)?;
        // let title = String::from_utf8_lossy(&buf).to_string();
        // let _update_time = reader.read_u64::<LE>()?;
        reader.seek(std::io::SeekFrom::Current(48))?;
        let len = reader.get_ref().len();
        let val = (len - reader.position() as usize) / width as usize / height as usize;
        let tile_format = if val == 12 {
            TileFormat::A
        } else if val == 14 {
            TileFormat::B
        } else if val == 15 {
            TileFormat::C
        } else if val == 36 {
            TileFormat::D
        } else {
            panic!("unknown tile format")
        };
        //构建图层
        //地面层
        // let tiles = Vec::with_capacity(width as usize * height as usize);
        let mut tilelayer = Layer::builder()
            .name("tilelayer")
            .width(width)
            .height(height)
            .build();

        //地表层
        let mut smtilelayer = Layer::builder()
            .name("smtilelayer")
            .width(width)
            .height(height)
            .build();
        //对象层，一般为静止对象，包括建筑物
        let mut objectlayer = Layer::builder()
            .name("objectlayer")
            .width(width)
            .height(height)
            .build();
        //门洞层，用于构建围栏
        let mut doorlayer = Layer::builder()
            .name("doorlayer")
            .width(width)
            .height(height)
            .build();
        //动画层
        let mut animationlayer = Layer::builder()
            .name("animationlayer")
            .width(width)
            .height(height)
            .build();
        //栅栏层（不能走）
        let mut barrierlayer = Layer::builder()
            .name("barrierlayer")
            .width(width)
            .height(height)
            .build();
        //禁飞区
        let mut fnzlayer = Layer::builder()
            .name("fnzlayer")
            .width(width)
            .height(height)
            .build();
        while let Ok(tile) = Tile::parse(&mut reader, tile_format) {
            let mut tilelayer_tile = None;
            let mut smtilelayer_tile = None;
            let mut objectlayer_tile = None;
            let mut animationlayer_tile = None;
            let mut doorlayer_tile = None;
            let mut barrierlayer_tile = None;
            let mut fnzlayer_tile = None;
            if tile.has_tile() {
                let tiled = Tiled::builder()
                    .id((tile.tile_file_name(), tile.tile()))
                    .build();
                tilelayer_tile = Some(tiled);
            }

            if tile.has_smtile() {
                let tiled = Tiled::builder()
                    .id((tile.smtile_file_name(), tile.smtile()))
                    .build();
                smtilelayer_tile = Some(tiled);
            }
            if tile.has_object() && tile.has_animation() {
                let tiled = Tiled::builder()
                    .id((tile.object_file_name(), tile.object()))
                    .properties(HashMap::from([
                        ("fps".into(), Value::U8(tile.animation_fps())),
                        ("frames".into(), Value::U8(tile.animation_frames())),
                        ("light".into(), Value::U8(tile.light)),
                    ]))
                    .build();
                // println!(
                //     "animation layer {}.{} frames({}) fps({})",
                //     tile.object_file_name(),
                //     tile.object(),
                //     tile.animation_frames(),
                //     tile.animation_fps()
                // );
                animationlayer_tile = Some(tiled);
            }
            if tile.has_object() && tile.has_door() && tile.is_door_open() {
                let tiled = Tiled::builder()
                    .id((
                        tile.object_file_name(),
                        tile.door() as i16 + tile.door_offset() as i16,
                    ))
                    .properties(HashMap::from([
                        ("door_offset".into(), Value::U8(tile.door_offset())),
                        ("is_open".into(), Value::Bool(tile.is_door_open())),
                        ("light".into(), Value::U8(tile.light)),
                    ]))
                    .build();
                // println!(
                //     "door layer {}.{} offset({}) is_open({})",
                //     tile.object_file_name(),
                //     tile.door(),
                //     tile.door_offset(),
                //     tile.is_door_open()
                // );
                doorlayer_tile = Some(tiled);
            }
            if tile.has_object() && !tile.has_animation() {
                let tiled = Tiled::builder()
                    .id((tile.object_file_name(), tile.object()))
                    .properties(HashMap::from([("light".into(), Value::U8(tile.light))]))
                    .build();
                objectlayer_tile = Some(tiled);
            }
            if !tile.can_walk() {
                let tiled = Tiled::builder().id(("", -1)).build();
                barrierlayer_tile = Some(tiled);
            }
            if !tile.can_fly() {
                let tiled = Tiled::builder().id(("", -1)).build();
                fnzlayer_tile = Some(tiled);
            }
            tilelayer.tiles.push(tilelayer_tile);
            smtilelayer.tiles.push(smtilelayer_tile);
            animationlayer.tiles.push(animationlayer_tile);
            doorlayer.tiles.push(doorlayer_tile);
            objectlayer.tiles.push(objectlayer_tile);
            barrierlayer.tiles.push(barrierlayer_tile);
            fnzlayer.tiles.push(fnzlayer_tile);
        }
        let map = Map::builder()
            .width(width)
            .height(height)
            .tile_format(tile_format)
            .layers(bon::map! {
                &tilelayer.name:tilelayer,
                &smtilelayer.name:smtilelayer,
                &objectlayer.name:objectlayer,
                &doorlayer.name:doorlayer,
                &animationlayer.name:animationlayer,
                &barrierlayer.name:barrierlayer,
                &fnzlayer.name:fnzlayer
            })
            .build();
        Ok(map)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    bg_image_idx: i16,
    mid_image_idx: i16,
    obj_image_idx: i16,
    door_idx: u8,
    door_offset: u8,
    animation_frames: u8,
    animation_tick: u8,
    obj_file_idx: u8,
    //亮度，一般为0/1/4
    light: u8,
    //tiles文件编号
    tile_file_idx: u8,
    //smtiles文件编号
    mid_file_idx: u8,
}

impl Tile {
    pub fn parse<R: Read + Seek>(reader: &mut R, fmt: TileFormat) -> anyhow::Result<Self> {
        let bg_image_idx = reader.read_i16::<LE>()?;
        let mid_image_idx = reader.read_i16::<LE>()?;
        let obj_image_idx = reader.read_i16::<LE>()?;
        let door_idx = reader.read_u8()?;
        let door_offset = reader.read_u8()?;
        let animation_frames = reader.read_u8()?;
        let animation_tick = reader.read_u8()?;
        let obj_file_idx = reader.read_u8()?;
        let light = reader.read_u8()?;

        let (tile_file_idx, mid_file_idx) = match fmt {
            TileFormat::A => (0, 0),
            TileFormat::B => {
                let tile_file_idx = reader.read_u8()?;
                let mid_file_idx = reader.read_u8()?;
                (tile_file_idx, mid_file_idx)
            }
            TileFormat::C => {
                let tile_file_idx = reader.read_u8()?;
                let mid_file_idx = reader.read_u8()?;
                reader.seek(std::io::SeekFrom::Current(1))?;
                (tile_file_idx, mid_file_idx)
            }
            TileFormat::D => {
                let tile_file_idx = reader.read_u8()?;
                let mid_file_idx = reader.read_u8()?;
                reader.seek(std::io::SeekFrom::Current(22))?;
                (tile_file_idx, mid_file_idx)
            }
        };

        Ok(Self {
            bg_image_idx,
            mid_image_idx,
            obj_image_idx,
            door_idx,
            door_offset,
            animation_frames,
            animation_tick,
            obj_file_idx,
            light,
            tile_file_idx,
            mid_file_idx,
        })
    }
}

impl Tile {
    pub fn tile(&self) -> i16 {
        (self.bg_image_idx & 0x7FFF) - 1
    }

    pub fn has_tile(&self) -> bool {
        self.bg_image_idx & 0x7FFF > 0 && self.bg_image_idx & 0x7FFF < i16::MAX
    }

    pub fn smtile(&self) -> i16 {
        (self.mid_image_idx & 0x7FFF) - 1
    }

    pub fn has_smtile(&self) -> bool {
        self.mid_image_idx & 0x7FFF > 0 && self.mid_image_idx & 0x7FFF < i16::MAX
    }

    pub fn object(&self) -> i16 {
        (self.obj_image_idx & 0x7FFF) - 1
    }

    pub fn has_object(&self) -> bool {
        self.obj_image_idx & 0x7fff > 0 && self.obj_image_idx & 0x7FFF < i16::MAX
    }

    //是否可行走
    pub fn can_walk(&self) -> bool {
        self.bg_image_idx as u16 & 0x8000 != 0x8000 && self.obj_image_idx as u16 & 0x8000 != 0x8000
    }

    pub fn can_fly(&self) -> bool {
        self.obj_image_idx as u16 & 0x8000 != 0x8000
    }

    pub fn door(&self) -> u8 {
        //后7位为门索引
        self.door_idx & 0x7F
    }

    pub fn has_door(&self) -> bool {
        //第一位为1表示有门
        (self.door_idx & 0x80) == 0x80
    }

    pub fn door_offset(&self) -> u8 {
        self.door_offset & 0x7F
    }

    pub fn is_door_open(&self) -> bool {
        //第一位为1表示门打开
        (self.door_offset & 0x80) == 0x80
    }

    pub fn animation_frames(&self) -> u8 {
        self.animation_frames & 0x7F
    }
    pub fn has_animation(&self) -> bool {
        (self.animation_frames & 0x80) == 0x80
    }

    pub fn animation_fps(&self) -> u8 {
        self.animation_tick
    }
    pub fn light(&self) -> u8 {
        self.light
    }
    pub fn tile_file_name(&self) -> String {
        if self.tile_file_idx == 0 {
            format!("{}/tiles", Map::MAP_DIR)
        } else {
            format!("{}/tiles{}", Map::MAP_DIR, self.tile_file_idx + 1)
        }
    }
    pub fn smtile_file_name(&self) -> String {
        if self.mid_file_idx == 0 {
            format!("{}/smtiles", Map::MAP_DIR)
        } else {
            format!("{}/smtiles{}", Map::MAP_DIR, self.mid_file_idx + 1)
        }
    }
    pub fn object_file_name(&self) -> String {
        if self.obj_file_idx == 0 {
            format!("{}/objects", Map::MAP_DIR)
        } else {
            format!("{}/objects{}", Map::MAP_DIR, self.obj_file_idx + 1)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub pixel_format: PixelFormat,
    pub idx: i16,
    pub width: u32,
    pub height: u32,
    pub opacity: u8,
    pub offset_x: i16,
    pub offset_y: i16,
    pub pixels: Vec<u8>,
}

impl Chunk {
    pub fn to_texture(&mut self, ctx: &mut ggez::Context) -> Texture {
        let mut rgba = vec![];
        for rgb in self.pixels.chunks(3) {
            let r = rgb[0];
            let g = rgb[1];
            let b = rgb[2];
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            //黑色背景设置为全透明
            if r == 0 && g == 0 && b == 0 {
                rgba.push(0);
            } else {
                rgba.push(255);
            }
        }
        // let mut image: ImageBuffer<Rgba<u8>, Vec<u8>> =
        //     ImageBuffer::from_raw(self.width, self.height, rgba).unwrap();

        // imageops::flip_vertical_in_place(&mut image);

        let image = ggez::graphics::Image::from_pixels(
            ctx,
            rgba.as_ref(),
            ggez::graphics::ImageFormat::Rgba8UnormSrgb,
            self.width,
            self.height,
        );

        // let  instances = graphics::InstanceArray::new(ctx, image);
        Texture {
            offset_x: self.offset_x,
            offset_y: self.offset_y,
            opacity: self.opacity,
            image,
        }
    }
}

impl Chunk {
    pub fn pixels(&self) -> &[u8] {
        &self.pixels[..]
    }

    #[inline]
    pub fn rgb888_to_rgb565((r5, g6, b5): (u8, u8, u8)) -> u16 {
        debug_assert!(r5 & 0b11111 == r5, "r5 channel too wide");
        debug_assert!(g6 & 0b111111 == g6, "g6 channel too wide");
        debug_assert!(b5 & 0b11111 == b5, "b5 channel too wide");
        (r5 as u16) << 11 | (g6 as u16) << 5 | b5 as u16
    }

    #[inline]
    pub fn rgb565_to_rgb888(packed: u16) -> (u8, u8, u8) {
        (
            ((packed & 0xf800) >> 8) as u8,
            ((packed & 0x07e0) >> 3) as u8,
            ((packed & 0x001f) << 3) as u8,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TileFormat {
    A, //12字节
    B, //14字节
    C, //15字节，韩国服
    D, //36字节，新地图
}

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    F32(f32),
    String(String),
}

impl Value {
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(v) => *v,
            Value::I8(v) => {
                if *v != 0 {
                    true
                } else {
                    false
                }
            }
            Value::U8(v) => {
                if *v != 0 {
                    true
                } else {
                    false
                }
            }
            Value::I16(v) => {
                if *v != 0 {
                    true
                } else {
                    false
                }
            }
            Value::U16(v) => {
                if *v != 0 {
                    true
                } else {
                    false
                }
            }
            Value::I32(v) => {
                if *v != 0 {
                    true
                } else {
                    false
                }
            }
            Value::U32(v) => {
                if *v != 0 {
                    true
                } else {
                    false
                }
            }
            Value::F32(v) => {
                if *v != 0.0 {
                    true
                } else {
                    false
                }
            }
            Value::String(v) => v.parse().unwrap_or(false),
        }
    }

    pub fn as_i8(&self) -> i8 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            Value::I8(v) => *v,
            Value::U8(v) => *v as i8,
            Value::I16(v) => *v as i8,
            Value::U16(v) => *v as i8,
            Value::I32(v) => *v as i8,
            Value::U32(v) => *v as i8,
            Value::F32(v) => *v as i8,
            Value::String(v) => v.parse().unwrap_or(-1),
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            Value::I8(v) => *v as u8,
            Value::U8(v) => *v,
            Value::I16(v) => *v as u8,
            Value::U16(v) => *v as u8,
            Value::I32(v) => *v as u8,
            Value::U32(v) => *v as u8,
            Value::F32(v) => *v as u8,
            Value::String(v) => v.parse().unwrap_or(0xFF),
        }
    }

    pub fn as_i16(&self) -> i16 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            Value::I8(v) => *v as i16,
            Value::U8(v) => *v as i16,
            Value::I16(v) => *v as i16,
            Value::U16(v) => *v as i16,
            Value::I32(v) => *v as i16,
            Value::U32(v) => *v as i16,
            Value::F32(v) => *v as i16,
            Value::String(v) => v.parse().unwrap_or(-1),
        }
    }

    pub fn as_u16(&self) -> u16 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            Value::I8(v) => *v as u16,
            Value::U8(v) => *v as u16,
            Value::I16(v) => *v as u16,
            Value::U16(v) => *v as u16,
            Value::I32(v) => *v as u16,
            Value::U32(v) => *v as u16,
            Value::F32(v) => *v as u16,
            Value::String(v) => v.parse().unwrap_or(0xFFFF),
        }
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            Value::I8(v) => *v as i32,
            Value::U8(v) => *v as i32,
            Value::I16(v) => *v as i32,
            Value::U16(v) => *v as i32,
            Value::I32(v) => *v as i32,
            Value::U32(v) => *v as i32,
            Value::F32(v) => *v as i32,
            Value::String(v) => v.parse().unwrap_or(-1),
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            }
            Value::I8(v) => *v as u32,
            Value::U8(v) => *v as u32,
            Value::I16(v) => *v as u32,
            Value::U16(v) => *v as u32,
            Value::I32(v) => *v as u32,
            Value::U32(v) => *v as u32,
            Value::F32(v) => *v as u32,
            Value::String(v) => v.parse().unwrap_or(0xFFFFFFFF),
        }
    }

    pub fn as_f32(&self) -> f32 {
        match self {
            Value::Bool(v) => {
                if *v {
                    1.
                } else {
                    0.
                }
            }
            Value::I8(v) => *v as f32,
            Value::U8(v) => *v as f32,
            Value::I16(v) => *v as f32,
            Value::U16(v) => *v as f32,
            Value::I32(v) => *v as f32,
            Value::U32(v) => *v as f32,
            Value::F32(v) => *v as f32,
            Value::String(v) => v.parse().unwrap_or(f32::MAX),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Bool(v) => v.to_string(),
            Value::I8(v) => v.to_string(),
            Value::U8(v) => v.to_string(),
            Value::I16(v) => v.to_string(),
            Value::U16(v) => v.to_string(),
            Value::I32(v) => v.to_string(),
            Value::U32(v) => v.to_string(),
            Value::F32(v) => v.to_string(),
            Value::String(v) => v.clone(),
        }
    }
}

pub struct Assets {
    assets: HashMap<String, Option<WZLReader>>,
}

impl Assets {
    pub fn new() -> Self {
        let assets = HashMap::new();
        Self { assets }
    }

    pub fn load_image(&mut self, tid: &TileId) -> anyhow::Result<Option<Chunk>> {
        if let Some(reader) = self.assets.get_mut(tid.tileset()) {
            if let Some(reader) = reader.as_mut() {
                return reader.load_image(tid.idx());
            } else {
                return Ok(None);
            }
        }
        if let Ok(mut reader) = WZLReader::with_cache(tid.tileset()) {
            let image = reader.load_image(tid.idx())?;
            self.assets.insert(tid.tileset().to_string(), Some(reader));
            return Ok(image);
        } else {
            self.assets.insert(tid.tileset().to_string(), None);
        }
        Ok(None)
    }
}

pub struct WZLReader {
    ptr: Vec<u32>,
    reader: BufReader<std::fs::File>,
    cache: HashMap<i16, Option<Chunk>>,
    cached: bool,
    path: String,
}

impl WZLReader {
    pub fn new(path: &str, cached: bool) -> anyhow::Result<Self> {
        println!("loading {}", path);
        let reader = std::fs::File::open(&format!("{path}.wzx"))?;
        let mut reader = BufReader::new(reader);
        reader.seek(std::io::SeekFrom::Start(44))?;
        let count = reader.read_u32::<LE>()? as usize;
        let mut ptr = Vec::with_capacity(count);
        for _ in 0..count {
            ptr.push(reader.read_u32::<LE>()?);
        }
        let reader = std::fs::File::open(&format!("{path}.wzl"))?;
        let mut reader = BufReader::new(reader);
        reader.seek(std::io::SeekFrom::Start(44))?;
        let wzl_count = reader.read_u32::<LE>()? as usize;
        assert!(wzl_count == count);
        Ok(Self {
            path: path.to_string(),
            ptr,
            reader,
            cache: HashMap::with_capacity(count),
            cached,
        })
    }

    pub fn with_cache(path: &str) -> anyhow::Result<Self> {
        Self::new(path, true)
    }
    pub fn with_nocache(path: &str) -> anyhow::Result<Self> {
        Self::new(path, false)
    }
}

impl WZLReader {
    pub fn load_all(&mut self) -> anyhow::Result<Vec<Option<Chunk>>> {
        let mut all = vec![];
        for i in 0..self.ptr.len() {
            let chunk = self.load_image(i as i16)?;
            all.push(chunk);
        }
        Ok(all)
    }
    pub fn load_image(&mut self, idx: i16) -> anyhow::Result<Option<Chunk>> {
        if self.cached {
            if let Some(image) = self.cache.get(&idx) {
                return Ok(image.clone());
            }
        }
        let mut offset = 0;
        if let Some(addr) = self.ptr.get(idx as usize) {
            offset = *addr;
        } else {
            println!(
                "unknown image index: {}.{idx}/{}",
                self.path,
                self.ptr.len()
            );
            return Ok(None);
        }
        // let offset = self.ptr[idx as usize];
        self.reader.seek(std::io::SeekFrom::Start(offset as _))?;
        let pixel_format = self.reader.read_u8()?;
        let opacity = self.reader.read_u8()?; //9表示透明度
        let _reserve = self.reader.read_u8()?;
        let _compress_level = self.reader.read_u8()?;
        let width = self.reader.read_u16::<LE>()?;
        let height = self.reader.read_u16::<LE>()?;
        let x = self.reader.read_i16::<LE>()?;
        let y = self.reader.read_i16::<LE>()?;
        let image_size = self.reader.read_u32::<LE>()?;

        if PixelFormat::try_from(pixel_format).is_err() || (width as usize * height as usize) < 4 {
            println!(
                "unknown image pixel format: {}.{idx}/{} pixel_format: {pixel_format} ({x},{y}).{width}x{height} image_size: {image_size}",
                self.path,
                self.ptr.len()
            );
            if self.cached {
                self.cache.insert(idx, None);
            }
            return Ok(None);
        }
        let pixel_format = pixel_format.try_into()?;
        let pixel_size = if let PixelFormat::Bit16 = pixel_format {
            width as usize * height as usize * 2
        } else {
            width as usize * height as usize
        };
        let mut pixels = if image_size == 0 {
            //表示没有压缩
            let mut pixels = vec![0u8; pixel_size];
            self.reader.read_exact(&mut pixels)?;
            pixels
        } else {
            let mut pixels = vec![0u8; image_size as usize];
            self.reader.read_exact(&mut pixels)?;
            let pixels = unzip(&pixels[..])?;
            pixels[0..pixel_size].to_vec()
        };

        let pixel_format = pixel_format.try_into()?;

        match pixel_format {
            PixelFormat::Bit16 => {
                //16位色深，R5G6B5
                if pixels.len() != width as usize * height as usize * 2 {
                    println!("{}={}x{}x2 ({},{})", pixels.len(), width, height, x, y);
                }
                assert!(pixels.len() == width as usize * height as usize * 2);
                let mut reader = std::io::Cursor::new(&pixels[..]);
                let mut rgb_pixels = vec![];
                while let Ok(pixel) = reader.read_u16::<LE>() {
                    let (r, g, b) = Chunk::rgb565_to_rgb888(pixel);
                    rgb_pixels.push(r);
                    rgb_pixels.push(g);
                    rgb_pixels.push(b);
                }
                pixels = rgb_pixels;
                // pixels = pixels.chunks(2).fold(vec![], |mut pixels, chunk| {
                //     //TODO 转裸指针避免数组越界检查
                //     let p = (chunk[1] as u16) << 8 | chunk[0] as u16;
                //     let (r, g, b) = Chunk::rgb565_to_rgb888(p);
                //     pixels.push(r);
                //     pixels.push(g);
                //     pixels.push(b);
                //     pixels
                // });
            }
            PixelFormat::Bit8 => {
                //8位色深，位图，用调色板调制成RGB
                assert!(pixels.len() == width as usize * height as usize);

                pixels = pixels.into_iter().fold(vec![], |mut pixels, idx| {
                    //TODO 转裸指针避免数组越界检查
                    let Color(r, g, b) = PALETTE[idx as usize];
                    pixels.push(r);
                    pixels.push(g);
                    pixels.push(b);
                    pixels
                });
            }
            PixelFormat::Bit24 | PixelFormat::Bit32 => {
                panic!("unsupported pixel format")
            }
        }
        //TODO GPU绘制时转
        // let mut image = RgbImage::from_vec(width as u32, height as u32, pixels.clone()).unwrap();
        // imageops::flip_vertical_in_place(&mut image);
        // pixels = image.to_vec();
        // let image = DynamicImage::ImageRgb8(
        //     RgbImage::from_vec(width as u32, height as u32, pixels)
        //         .ok_or(anyhow::anyhow!("Failed to create image from chunk"))?,
        // );
        let chunk = Chunk {
            idx,
            pixel_format,
            opacity,
            width: width as u32,
            height: height as u32,
            offset_x: x,
            offset_y: y,
            pixels,
        };
        if self.cached {
            self.cache.insert(idx, Some(chunk.clone()));
        }
        Ok(Some(chunk))
    }
}

fn unzip(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut z = ZlibDecoder::new(Vec::new());
    z.write_all(&bytes[..])?;
    let fin = z.finish()?;
    Ok(fin)
}

// 对于bmp图片填充字节进行补足
//
// @param bitCount 每行图片色彩值字节位数(bit)
// @return 每行图片色彩数据占字节数(byte)
pub const fn width_bytes(bit_count: i32) -> i32 {
    (bit_count + 31) / 32 * 4
}

/**
 * 计算bmp图片逐行读取时需要跳过的字节数
 * <br>
 * 即用该行实际占用的字节数减去真正占用的字节数
 *
 * @param bit 位深度
 * @param width 图片宽度
 * @return 读取某行数据时需要跳过的字节数
 */
pub const fn skip_bytes(bit: i32, width: i32) -> i32 {
    return width_bytes(bit * width) - width * (bit / 8);
}

#[derive(Debug, Clone, Copy)]
pub enum PixelFormat {
    Bit8,  //BITMAP
    Bit16, //R5G6B5
    Bit24, //R8G8B8
    Bit32, //R8G8B8A8
}

impl TryFrom<u8> for PixelFormat {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            3 => Ok(PixelFormat::Bit8),
            5 => Ok(PixelFormat::Bit16),
            7 => Ok(PixelFormat::Bit24),
            9 => Ok(PixelFormat::Bit32),
            _ => Err(anyhow::anyhow!("invalid pixel format {value}")),
        }
    }
}
