use ggez::{
    graphics::{self, Drawable, Rect},
    Context,
};
use std::hash::Hash;

use crate::ui;

impl<T: Copy + Eq + Hash> ui::UiContent<T> for ggez::graphics::Image {
    fn to_element_builder(self, id: u32, ctx: &Context) -> ui::UiElementBuilder<T>
    where
        Self: Sized + 'static,
    {
        let size = self.dimensions(&ctx.gfx);

        ui::UiElementBuilder::new(id, self)
            .with_size(
                ui::Size::Fill(size.w, f32::INFINITY),
                ui::Size::Fill(size.h, f32::INFINITY),
            )
            .with_preserve_ratio(true)
    }

    fn draw_content(
        &mut self,
        ctx: &mut Context,
        canvas: &mut graphics::Canvas,
        param: ui::UiDrawParam,
    ) {
        let dim = self.dimensions(ctx);
        canvas.draw(
            self,
            param.param.dest_rect(Rect::new(
                param.target.x,
                param.target.y,
                param.target.w / dim.w,
                param.target.h / dim.h,
            )),
        );
    }
}
