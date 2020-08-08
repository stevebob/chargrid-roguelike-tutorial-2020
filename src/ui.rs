use crate::app::colours;
use crate::game::{ExamineCell, LogMessage};
use crate::world::HitPoints;
use chargrid::{
    decorator::{AlignView, Alignment, AlignmentX, AlignmentY, BoundView},
    render::{ColModify, Frame, Style, View, ViewCell, ViewContext},
    text::{wrap, RichTextPartOwned, RichTextViewSingleLine, StringView, StringViewSingleLine},
};
use coord_2d::{Coord, Size};
use rgb24::Rgb24;

const HEALTH_WIDTH: u32 = 10;
const HEALTH_FILL_COLOUR: Rgb24 = Rgb24::new(200, 0, 0);
const HEALTH_EMPTY_COLOUR: Rgb24 = Rgb24::new(100, 0, 0);

#[derive(Default)]
struct HealthView {
    buf: String,
}

impl View<HitPoints> for HealthView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        hit_points: HitPoints,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        use std::fmt::Write;
        self.buf.clear();
        write!(&mut self.buf, "{}/{}", hit_points.current, hit_points.max).unwrap();
        let mut hit_points_text_view = BoundView {
            size: Size::new(HEALTH_WIDTH, 1),
            view: AlignView {
                alignment: Alignment::centre(),
                view: StringViewSingleLine::new(Style::new().with_foreground(Rgb24::new_grey(255))),
            },
        };
        hit_points_text_view.view(&self.buf, context.add_depth(1), frame);
        let mut health_fill_width = (hit_points.current * HEALTH_WIDTH) / hit_points.max;
        if hit_points.current > 0 {
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

struct MessagesView {
    buf: Vec<RichTextPartOwned>,
}

impl Default for MessagesView {
    fn default() -> Self {
        let common = RichTextPartOwned::new(String::new(), Style::new());
        Self {
            buf: vec![common.clone(), common.clone(), common],
        }
    }
}

impl<'a> View<&'a [LogMessage]> for MessagesView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        messages: &'a [LogMessage],
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        fn format_message(buf: &mut [RichTextPartOwned], message: LogMessage) {
            use std::fmt::Write;
            use LogMessage::*;
            buf[0].text.clear();
            buf[1].text.clear();
            buf[2].text.clear();
            buf[0].style.foreground = Some(Rgb24::new_grey(255));
            buf[1].style.bold = Some(true);
            buf[2].style.foreground = Some(Rgb24::new_grey(255));
            match message {
                PlayerAttacksNpc(npc_type) => {
                    write!(&mut buf[0].text, "You attack the ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, ".").unwrap();
                }
                NpcAttacksPlayer(npc_type) => {
                    write!(&mut buf[0].text, "The ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, " attacks you.").unwrap();
                }
                PlayerKillsNpc(npc_type) => {
                    write!(&mut buf[0].text, "You kill the ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, ".").unwrap();
                }
                NpcKillsPlayer(npc_type) => {
                    write!(&mut buf[0].text, "THE ").unwrap();
                    buf[0].style.foreground = Some(Rgb24::new(255, 0, 0));
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].text.make_ascii_uppercase();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, " KILLS YOU!").unwrap();
                    buf[2].style.foreground = Some(Rgb24::new(255, 0, 0));
                }
                PlayerGets(item_type) => {
                    write!(&mut buf[0].text, "You get the ").unwrap();
                    write!(&mut buf[1].text, "{}", item_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::item_colour(item_type));
                    write!(&mut buf[2].text, ".").unwrap();
                }
                PlayerInventoryIsFull => {
                    write!(&mut buf[0].text, "Inventory is full!").unwrap();
                }
                NoItemUnderPlayer => {
                    write!(&mut buf[0].text, "Nothing to get!").unwrap();
                }
                NoItemInInventorySlot => {
                    write!(&mut buf[0].text, "No item in inventory slot!").unwrap();
                }
                PlayerHeals => {
                    write!(&mut buf[0].text, "You feel slightly better.").unwrap();
                    buf[0].style.foreground = Some(Rgb24::new(0, 187, 0));
                }
                PlayerDrops(item_type) => {
                    write!(&mut buf[0].text, "You drop the ").unwrap();
                    write!(&mut buf[1].text, "{}", item_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::item_colour(item_type));
                    write!(&mut buf[2].text, ".").unwrap();
                }
                NoSpaceToDropItem => {
                    write!(&mut buf[0].text, "No space to drop item!").unwrap();
                }
                PlayerLaunchesProjectile(projectile) => {
                    write!(&mut buf[0].text, "You launch a ").unwrap();
                    write!(&mut buf[1].text, "{}", projectile.name()).unwrap();
                    buf[1].style.foreground = Some(colours::projectile_colour(projectile));
                    write!(&mut buf[2].text, "!").unwrap();
                }
                NpcDies(npc_type) => {
                    write!(&mut buf[0].text, "The ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, " dies.").unwrap();
                }
                NpcBecomesConfused(npc_type) => {
                    write!(&mut buf[0].text, "The ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, " is confused.").unwrap();
                }
                NpcIsNoLongerConfused(npc_type) => {
                    write!(&mut buf[0].text, "The ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, "'s confusion passes.").unwrap();
                }
                PlayerDodges(npc_type) => {
                    write!(&mut buf[0].text, "You dodge the ").unwrap();
                    write!(&mut buf[1].text, "{}'s", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, " attack.").unwrap();
                }
                NpcDodges(npc_type) => {
                    write!(&mut buf[0].text, "The ").unwrap();
                    write!(&mut buf[1].text, "{}", npc_type.name()).unwrap();
                    buf[1].style.foreground = Some(colours::npc_colour(npc_type));
                    write!(&mut buf[2].text, " dodges your attack.").unwrap();
                }
            }
        }
        const NUM_MESSAGES: usize = 4;
        let start_index = messages.len().saturating_sub(NUM_MESSAGES);
        for (i, &message) in (&messages[start_index..]).iter().enumerate() {
            format_message(&mut self.buf, message);
            let offset = Coord::new(0, i as i32);
            RichTextViewSingleLine.view(
                self.buf.iter().map(|part| part.as_rich_text_part()),
                context.add_offset(offset),
                frame,
            );
        }
    }
}

fn examine_cell_str(examine_cell: ExamineCell) -> &'static str {
    match examine_cell {
        ExamineCell::Npc(npc_type) | ExamineCell::NpcCorpse(npc_type) => npc_type.name(),
        ExamineCell::Item(item_type) => item_type.name(),
        ExamineCell::Player => "yourself",
    }
}

#[derive(Default)]
struct StatsView {
    buf: String,
}

pub struct StatsData {
    pub strength: i32,
    pub dexterity: i32,
    pub intelligence: i32,
}

impl<'a> View<&'a StatsData> for StatsView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        data: &'a StatsData,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        use std::fmt::Write;
        self.buf.clear();
        write!(
            &mut self.buf,
            "str: {}, dex: {}, int: {}",
            data.strength, data.dexterity, data.intelligence
        )
        .unwrap();
        StringViewSingleLine::new(Style::new().with_foreground(Rgb24::new_grey(187)))
            .view(&self.buf, context, frame);
    }
}

#[derive(Default)]
struct DungeonLevelView {
    buf: String,
}

impl View<u32> for DungeonLevelView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        dungeon_level: u32,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        use std::fmt::Write;
        self.buf.clear();
        write!(&mut self.buf, "Level: {}", dungeon_level).unwrap();
        StringViewSingleLine::new(Style::new().with_foreground(Rgb24::new_grey(187)))
            .view(&self.buf, context, frame);
    }
}

pub struct UiData<'a> {
    pub player_hit_points: HitPoints,
    pub messages: &'a [LogMessage],
    pub name: Option<&'static str>,
    pub examine_cell: Option<ExamineCell>,
    pub stats_data: StatsData,
    pub dungeon_level: u32,
}

#[derive(Default)]
pub struct UiView {
    health_view: HealthView,
    messages_view: MessagesView,
    stats_view: StatsView,
    dungeon_level_view: DungeonLevelView,
}

fn centre_health_width<T: Clone>(view: impl View<T>, height: u32) -> impl View<T> {
    BoundView {
        size: Size::new(HEALTH_WIDTH, height),
        view: AlignView {
            alignment: Alignment {
                x: AlignmentX::Centre,
                y: AlignmentY::Bottom,
            },
            view,
        },
    }
}

impl<'a> View<UiData<'a>> for UiView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        data: UiData,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        self.health_view
            .view(data.player_hit_points, context, frame);
        self.stats_view.view(
            &data.stats_data,
            context.add_offset(Coord::new(HEALTH_WIDTH as i32 + 1, 0)),
            frame,
        );
        centre_health_width(&mut self.dungeon_level_view, 1).view(
            data.dungeon_level,
            context.add_offset(Coord::new(0, 1)),
            frame,
        );
        let message_log_offset = Coord::new(HEALTH_WIDTH as i32 + 1, 1);
        self.messages_view
            .view(data.messages, context.add_offset(message_log_offset), frame);
        if let Some(name) = data.name {
            BoundView {
                size: Size::new(HEALTH_WIDTH, 1),
                view: AlignView {
                    alignment: Alignment::centre(),
                    view: StringViewSingleLine::new(
                        Style::new().with_foreground(Rgb24::new_grey(255)),
                    ),
                },
            }
            .view(name, context.add_offset(Coord::new(0, 2)), frame);
        }
        if let Some(examine_cell) = data.examine_cell {
            centre_health_width(
                StringView::new(
                    Style::new().with_foreground(Rgb24::new_grey(187)),
                    wrap::Word::new(),
                ),
                2,
            )
            .view(
                examine_cell_str(examine_cell),
                context.add_offset(Coord::new(0, 3)),
                frame,
            );
        }
    }
}
