//! C# `GrayScale` 等价灰度绘制（`Client/Data/Shaders/grayscale.ps`）。
//!
//! C# 像素着色器（ps_1_2）：
//! ```text
//! def c0, 0.3, 0.59, 0.11, 1
//! tex t0
//! dp3 r0, t0, c0      ; 亮度 = 0.3R + 0.59G + 0.11B
//! mov r0.a, t0        ; alpha 原样
//! ```
//! `MirImageControl.DrawControl` 在控件 `GrayScale == true` 时 `DXManager.SetGrayscale(true)`，
//! 即**该控件自身**按灰度绘制（原版只在 Craft `CraftButton`、TrustMerchant 底栏四键、
//! Mail 两个禁用占位键上显式置真；Creature 操作按钮并没有置真）。
//!
//! Bevy 侧等价实现：把源精灵图按同一公式（8bit 空间取整、alpha 不变）烘一张灰度变体并缓存，
//! 由 [`UiGray`] + [`apply_ui_gray_system`] 在每帧帧切换之后替换 `ImageNode.image`。

use std::collections::HashMap;

use bevy::prelude::*;

use crate::map_renderer::make_image;

/// C# `grayscale.ps` 的 `c0` 系数
pub const GRAY_COEFFS: [f32; 3] = [0.3, 0.59, 0.11];

/// 单像素灰度：`dp3` 语义（8bit 输入 → 亮度取整 → 8bit 输出），alpha 不变。
pub fn gray_pixel([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let lum = GRAY_COEFFS[0] * r as f32 + GRAY_COEFFS[1] * g as f32 + GRAY_COEFFS[2] * b as f32;
    let l = lum.round().clamp(0.0, 255.0) as u8;
    [l, l, l, a]
}

/// 整幅 RGBA8 灰度化（长度必须是 4 的倍数）
pub fn gray_rgba(data: &[u8]) -> Vec<u8> {
    data.chunks_exact(4)
        .flat_map(|px| gray_pixel([px[0], px[1], px[2], px[3]]))
        .collect()
}

/// 灰度图缓存（源 handle ↔ 灰度变体 handle）。
#[derive(Resource, Default)]
pub struct UiGrayCache {
    gray: HashMap<Handle<Image>, Handle<Image>>,
    base: HashMap<Handle<Image>, Handle<Image>>,
}

impl UiGrayCache {
    /// 源图 → 灰度变体（首次调用时生成并缓存；源图缺失/非 RGBA8 返回 None）。
    pub fn gray_handle(
        &mut self,
        images: &mut Assets<Image>,
        src: &Handle<Image>,
    ) -> Option<Handle<Image>> {
        if let Some(h) = self.gray.get(src) {
            return Some(h.clone());
        }
        let image = images.get(src)?;
        let data = image.data.as_ref()?;
        let (w, h) = (image.width(), image.height());
        if data.len() != (w as usize) * (h as usize) * 4 {
            // 只处理 RGBA8（UI 精灵库统一 `make_image` = Rgba8UnormSrgb）
            return None;
        }
        let gray = images.add(make_image(gray_rgba(data), w, h));
        self.gray.insert(src.clone(), gray.clone());
        self.base.insert(gray.clone(), src.clone());
        Some(gray)
    }

    /// 灰度变体 → 源图；传入非灰度 handle 时原样返回（其它系统刚写回的原帧）。
    pub fn base_handle(&self, handle: &Handle<Image>) -> Handle<Image> {
        self.base
            .get(handle)
            .cloned()
            .unwrap_or_else(|| handle.clone())
    }

    /// 已缓存条目数（测试用）
    pub fn len(&self) -> usize {
        self.gray.len()
    }

    pub fn is_empty(&self) -> bool {
        self.gray.is_empty()
    }
}

/// C# `MirImageControl.GrayScale`：`gray == true` 时该 UI 节点按灰度绘制。
#[derive(Component, Default)]
pub struct UiGray {
    pub gray: bool,
}

impl UiGray {
    pub fn new(gray: bool) -> Self {
        Self { gray }
    }
}

/// 应用 [`UiGray`]（须在 `image_button_system` 等帧切换系统之后运行：
/// 后者每帧把原帧写回 `ImageNode.image`，本系统再按需替换成灰度变体）。
pub fn apply_ui_gray_system(
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiGrayCache>,
    mut q: Query<(&UiGray, &mut ImageNode)>,
) {
    for (flag, mut node) in &mut q {
        let base = cache.base_handle(&node.image);
        let want = if flag.gray {
            match cache.gray_handle(&mut images, &base) {
                Some(g) => g,
                None => base,
            }
        } else {
            base
        };
        if node.image != want {
            node.image = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2742：灰度公式 = C# `grayscale.ps` 的 `dp3 c0=(0.3,0.59,0.11)`
    #[test]
    fn gray_pixel_matches_csharp_shader() {
        assert_eq!(gray_pixel([255, 255, 255, 255]), [255, 255, 255, 255]);
        assert_eq!(gray_pixel([0, 0, 0, 200]), [0, 0, 0, 200]);
        // 0.3*255 = 76.5 → 77（四舍五入，对齐 GPU UNORM 写回）
        assert_eq!(gray_pixel([255, 0, 0, 255]), [77, 77, 77, 255]);
        // 0.59*255 = 150.45 → 150
        assert_eq!(gray_pixel([0, 255, 0, 255]), [150, 150, 150, 255]);
        // 0.11*255 = 28.05 → 28
        assert_eq!(gray_pixel([0, 0, 255, 255]), [28, 28, 28, 255]);
        // 混合 + alpha 保留
        let [l, _, _, a] = gray_pixel([100, 200, 50, 7]);
        let expect: f32 = GRAY_COEFFS[0] * 100.0 + GRAY_COEFFS[1] * 200.0 + GRAY_COEFFS[2] * 50.0;
        assert_eq!(l, expect.round() as u8);
        assert_eq!(a, 7);
    }

    /// 整图灰度保持尺寸与 alpha 通道
    #[test]
    fn gray_rgba_keeps_size_and_alpha() {
        let src = vec![255, 0, 0, 255, 0, 255, 0, 128];
        let out = gray_rgba(&src);
        assert_eq!(out.len(), src.len());
        assert_eq!(&out[0..4], &[77, 77, 77, 255]);
        assert_eq!(&out[4..8], &[150, 150, 150, 128]);
    }

    /// 缓存：同一源图只烘一次；灰度 handle 可回溯到源图（帧切换系统每帧写回原帧）。
    #[test]
    fn gray_cache_round_trips_and_dedups() {
        let mut images = Assets::<Image>::default();
        let src = images.add(make_image(vec![255, 0, 0, 255], 1, 1));
        let mut cache = UiGrayCache::default();

        let gray = cache
            .gray_handle(&mut images, &src)
            .expect("应生成灰度变体");
        assert_ne!(gray, src);
        assert_eq!(cache.base_handle(&gray), src);
        // 非灰度 handle 原样返回
        assert_eq!(cache.base_handle(&src), src);
        // 第二次取同源 → 复用缓存
        let again = cache.gray_handle(&mut images, &src).unwrap();
        assert_eq!(again, gray);
        assert_eq!(cache.len(), 1);
        // 变体像素 = 灰度公式结果
        let data = images.get(&gray).unwrap().data.clone().unwrap();
        assert_eq!(data, vec![77u8, 77, 77, 255]);
    }
}
