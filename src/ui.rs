use crate::world::HitPoints;
use chargrid::{
    decorator::{AlignView, Alignment, BoundView},
    render::{ColModify, Frame, Style, View, ViewCell, ViewContext},
    text::StringViewSingleLine,
};
use coord_2d::{Coord, Size};
use rgb24::Rgb24;

const HEALTH_WIDTH: u32 = 10;
const HEALTH_FILL_COLOUR: Rgb24 = Rgb24::new(200, 0, 0);
const HEALTH_EMPTY_COLOUR: Rgb24 = Rgb24::new(100, 0, 0);

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
        let mut hit_points_text_view = BoundView {
            size: Size::new(HEALTH_WIDTH, 1),
            view: AlignView {
                alignment: Alignment::centre(),
                view: StringViewSingleLine::new(Style::new().with_foreground(Rgb24::new_grey(255))),
            },
        };
        hit_points_text_view.view(&self.buf, context.add_depth(1), frame);
        let mut health_fill_width =
            (data.player_hit_points.current * HEALTH_WIDTH) / data.player_hit_points.max;
        if data.player_hit_points.current > 0 {
            health_fill_width = health_fill_width.max(1);
        }
        for i in 0..health_fill_width {
            frame.set_cell_relative(
                Coord::new(i as i32, 0),
                0,
                ViewCell::new().with_background(HEALTH_FILL_COLOUR),
                context,
            );
        }
        for i in health_fill_width..HEALTH_WIDTH {
            frame.set_cell_relative(
                Coord::new(i as i32, 0),
                0,
                ViewCell::new().with_background(HEALTH_EMPTY_COLOUR),
                context,
            );
        }
    }
}
