use crate::world::HitPoints;
use chargrid::{
    render::{ColModify, Frame, Style, View, ViewContext},
    text::StringViewSingleLine,
};
use rgb24::Rgb24;

pub struct UiData {
    pub player_hit_points: HitPoints,
}

#[derive(Default)]
pub struct UiView {
    buf: String,
}

impl View<UiData> for UiView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        data: UiData,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        use std::fmt::Write;
        self.buf.clear();
        write!(
            &mut self.buf,
            "{}/{}",
            data.player_hit_points.current, data.player_hit_points.max
        )
        .unwrap();
        StringViewSingleLine::new(Style::new().with_foreground(Rgb24::new_grey(255)))
            .view(&self.buf, context, frame);
    }
}
