use crate::game::GameState;
use crate::ui::{UiData, UiView};
use crate::visibility::{CellVisibility, VisibilityAlgorithm};
use crate::world::{ItemType, Layer, NpcType, Tile};
use chargrid::{
    app::App as ChargridApp,
    decorator::{
        AlignView, Alignment, BorderStyle, BorderView, BoundView, FillBackgroundView, MinSizeView,
    },
    event_routine::{
        self, common_event::CommonEvent, make_either, DataSelector, Decorate, EventOrPeek,
        EventRoutine, EventRoutineView, Handled, Loop, SideEffect, Value, ViewSelector,
    },
    input::{keys, Input, KeyboardInput},
    menu::{
        self, ChooseSelector, MenuIndexFromScreenCoord, MenuInstanceBuilder, MenuInstanceChoose,
        MenuInstanceChooseOrEscape, MenuInstanceMouseTracker, MenuInstanceRoutine,
    },
    render::{ColModify, ColModifyMap, Frame, Style, View, ViewCell, ViewContext},
    text::{RichTextPart, RichTextViewSingleLine},
};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use rgb24::Rgb24;
use std::collections::HashMap;

const UI_NUM_ROWS: u32 = 5;

#[derive(Clone, Copy, Debug)]
struct InventorySlotMenuEntry {
    index: usize,
    key: char,
}

struct InventorySlotMenuSelect;

impl ChooseSelector for InventorySlotMenuSelect {
    type ChooseOutput = MenuInstanceChooseOrEscape<InventorySlotMenuEntry>;
    fn choose_mut<'a>(&self, input: &'a mut Self::DataInput) -> &'a mut Self::ChooseOutput {
        &mut input.inventory_slot_menu
    }
}

impl DataSelector for InventorySlotMenuSelect {
    type DataInput = AppData;
    type DataOutput = AppData;
    fn data<'a>(&self, input: &'a Self::DataInput) -> &'a Self::DataOutput {
        input
    }
    fn data_mut<'a>(&self, input: &'a mut Self::DataInput) -> &'a mut Self::DataOutput {
        input
    }
}

impl ViewSelector for InventorySlotMenuSelect {
    type ViewInput = AppView;
    type ViewOutput = InventorySlotMenuView;
    fn view<'a>(&self, input: &'a Self::ViewInput) -> &'a Self::ViewOutput {
        &input.inventory_slot_menu_view
    }
    fn view_mut<'a>(&self, input: &'a mut Self::ViewInput) -> &'a mut Self::ViewOutput {
        &mut input.inventory_slot_menu_view
    }
}

struct InventorySlotMenuDecorate<'a> {
    title: &'a str,
}

impl<'a> Decorate for InventorySlotMenuDecorate<'a> {
    type View = AppView;
    type Data = AppData;
    fn view<E, F, C>(
        &self,
        data: &Self::Data,
        mut event_routine_view: EventRoutineView<E>,
        context: ViewContext<C>,
        frame: &mut F,
    ) where
        E: EventRoutine<Data = Self::Data, View = Self::View>,
        F: Frame,
        C: ColModify,
    {
        BoundView {
            size: data.game_state.size(),
            view: AlignView {
                alignment: Alignment::centre(),
                view: FillBackgroundView {
                    rgb24: Rgb24::new_grey(0),
                    view: BorderView {
                        style: &BorderStyle {
                            title: Some(self.title.to_string()),
                            title_style: Style::new().with_foreground(Rgb24::new_grey(255)),
                            ..Default::default()
                        },
                        view: MinSizeView {
                            size: Size::new(12, 0),
                            view: &mut event_routine_view,
                        },
                    },
                },
            },
        }
        .view(data, context.add_depth(10), frame);
        event_routine_view.view.game_view.view(
            &data.game_state,
            context.compose_col_modify(ColModifyMap(|c: Rgb24| c.saturating_scalar_mul_div(1, 2))),
            frame,
        );
        event_routine_view.view.render_ui(&data, context, frame);
    }
}

#[derive(Default)]
struct InventorySlotMenuView {
    mouse_tracker: MenuInstanceMouseTracker,
}

impl MenuIndexFromScreenCoord for InventorySlotMenuView {
    fn menu_index_from_screen_coord(&self, len: usize, coord: Coord) -> Option<usize> {
        self.mouse_tracker.menu_index_from_screen_coord(len, coord)
    }
}

impl<'a> View<&'a AppData> for InventorySlotMenuView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        data: &'a AppData,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        let player_inventory_slots = data.game_state.player_inventory().slots();
        self.mouse_tracker.new_frame(context.offset);
        for ((i, entry, maybe_selected), &slot) in data
            .inventory_slot_menu
            .menu_instance()
            .enumerate()
            .zip(player_inventory_slots.into_iter())
        {
            let (name, name_colour) = if let Some(item_entity) = slot {
                let item_type = data
                    .game_state
                    .item_type(item_entity)
                    .expect("non-item in player inventory");
                (item_type.name(), colours::item_colour(item_type))
            } else {
                ("-", Rgb24::new_grey(187))
            };
            let (selected_prefix, prefix_style, name_style) = if maybe_selected.is_some() {
                (
                    ">",
                    Style::new()
                        .with_foreground(Rgb24::new_grey(255))
                        .with_bold(true),
                    Style::new().with_foreground(name_colour).with_bold(true),
                )
            } else {
                (
                    " ",
                    Style::new().with_foreground(Rgb24::new_grey(187)),
                    Style::new().with_foreground(name_colour.saturating_scalar_mul_div(2, 3)),
                )
            };
            let prefix = format!("{} {}) ", selected_prefix, entry.key);
            let text = &[
                RichTextPart {
                    text: &prefix,
                    style: prefix_style,
                },
                RichTextPart {
                    text: name,
                    style: name_style,
                },
            ];
            let size = RichTextViewSingleLine::new().view_size(
                text.into_iter().cloned(),
                context.add_offset(Coord::new(0, i as i32)),
                frame,
            );
            self.mouse_tracker.on_entry_view_size(size);
        }
    }
}

fn inventory_slot_menu<'a>(
    title: &'a str,
) -> impl 'a
       + EventRoutine<
    Return = Result<InventorySlotMenuEntry, menu::Escape>,
    Data = AppData,
    View = AppView,
    Event = CommonEvent,
> {
    MenuInstanceRoutine::new(InventorySlotMenuSelect)
        .convert_input_to_common_event()
        .decorated(InventorySlotMenuDecorate { title })
}

struct GameEventRoutine;

enum GameReturn {
    Exit,
    UseItem,
    DropItem,
    GameOver,
}

impl EventRoutine for GameEventRoutine {
    type Return = GameReturn;
    type Data = AppData;
    type View = AppView;
    type Event = CommonEvent;

    fn handle<EP>(
        self,
        data: &mut Self::Data,
        _view: &Self::View,
        event_or_peek: EP,
    ) -> Handled<Self::Return, Self>
    where
        EP: EventOrPeek<Event = Self::Event>,
    {
        event_routine::event_or_peek_with_handled(event_or_peek, self, |s, event| match event {
            CommonEvent::Input(input) => {
                if let Some(game_return) = data.handle_input(input) {
                    Handled::Return(game_return)
                } else {
                    Handled::Continue(s)
                }
            }
            CommonEvent::Frame(_period) => Handled::Continue(s),
        })
    }

    fn view<F, C>(
        &self,
        data: &Self::Data,
        view: &mut Self::View,
        context: ViewContext<C>,
        frame: &mut F,
    ) where
        F: Frame,
        C: ColModify,
    {
        view.game_view.view(&data.game_state, context, frame);
        view.render_ui(&data, context, frame);
    }
}

struct AppData {
    game_state: GameState,
    visibility_algorithm: VisibilityAlgorithm,
    inventory_slot_menu: MenuInstanceChooseOrEscape<InventorySlotMenuEntry>,
}

impl AppData {
    fn new(screen_size: Size, rng_seed: u64, visibility_algorithm: VisibilityAlgorithm) -> Self {
        let game_area_size = screen_size.set_height(screen_size.height() - UI_NUM_ROWS);
        let game_state = GameState::new(game_area_size, rng_seed, visibility_algorithm);
        let player_inventory = game_state.player_inventory();
        let inventory_slot_menu = {
            let items = (0..player_inventory.slots().len())
                .zip('a'..)
                .map(|(index, key)| InventorySlotMenuEntry { index, key })
                .collect::<Vec<_>>();
            let hotkeys = items
                .iter()
                .map(|&entry| (entry.key, entry))
                .collect::<HashMap<_, _>>();
            MenuInstanceBuilder {
                items,
                hotkeys: Some(hotkeys),
                selected_index: 0,
            }
            .build()
            .unwrap()
            .into_choose_or_escape()
        };
        Self {
            game_state,
            visibility_algorithm,
            inventory_slot_menu,
        }
    }
    fn handle_input(&mut self, input: Input) -> Option<GameReturn> {
        if !self.game_state.is_player_alive() {
            return Some(GameReturn::GameOver);
        }
        match input {
            Input::Keyboard(key) => match key {
                KeyboardInput::Left => self.game_state.maybe_move_player(CardinalDirection::West),
                KeyboardInput::Right => self.game_state.maybe_move_player(CardinalDirection::East),
                KeyboardInput::Up => self.game_state.maybe_move_player(CardinalDirection::North),
                KeyboardInput::Down => self.game_state.maybe_move_player(CardinalDirection::South),
                KeyboardInput::Char(' ') => self.game_state.wait_player(),
                KeyboardInput::Char('g') => self.game_state.maybe_player_get_item(),
                KeyboardInput::Char('i') => return Some(GameReturn::UseItem),
                KeyboardInput::Char('d') => return Some(GameReturn::DropItem),
                keys::ESCAPE => return Some(GameReturn::Exit),
                _ => (),
            },
            _ => (),
        }
        self.game_state.update_visibility(self.visibility_algorithm);
        None
    }
}

struct AppView {
    ui_y_offset: i32,
    game_view: GameView,
    ui_view: UiView,
    inventory_slot_menu_view: InventorySlotMenuView,
}

impl AppView {
    fn new(screen_size: Size) -> Self {
        const UI_Y_PADDING: u32 = 1;
        let ui_y_offset = (screen_size.height() - UI_NUM_ROWS + UI_Y_PADDING) as i32;
        Self {
            ui_y_offset,
            game_view: GameView::default(),
            ui_view: UiView::default(),
            inventory_slot_menu_view: InventorySlotMenuView::default(),
        }
    }
    fn render_ui<F: Frame, C: ColModify>(
        &mut self,
        data: &AppData,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        let player_hit_points = data.game_state.player_hit_points();
        let messages = data.game_state.message_log();
        self.ui_view.view(
            UiData {
                player_hit_points,
                messages,
            },
            context.add_offset(Coord::new(0, self.ui_y_offset)),
            frame,
        );
    }
}

pub mod colours {
    use super::*;
    pub const PLAYER: Rgb24 = Rgb24::new_grey(255);
    pub const ORC: Rgb24 = Rgb24::new(0, 187, 0);
    pub const TROLL: Rgb24 = Rgb24::new(187, 0, 0);
    pub const HEALTH_POTION: Rgb24 = Rgb24::new(255, 0, 255);

    pub fn npc_colour(npc_type: NpcType) -> Rgb24 {
        match npc_type {
            NpcType::Orc => ORC,
            NpcType::Troll => TROLL,
        }
    }

    pub fn item_colour(item_type: ItemType) -> Rgb24 {
        match item_type {
            ItemType::HealthPotion => HEALTH_POTION,
        }
    }
}

fn currently_visible_view_cell_of_tile(tile: Tile) -> ViewCell {
    match tile {
        Tile::Player => ViewCell::new()
            .with_character('@')
            .with_foreground(colours::PLAYER),
        Tile::PlayerCorpse => ViewCell::new()
            .with_character('%')
            .with_foreground(colours::PLAYER),
        Tile::Floor => ViewCell::new()
            .with_character('.')
            .with_foreground(Rgb24::new_grey(63))
            .with_background(Rgb24::new(0, 0, 63)),
        Tile::Wall => ViewCell::new()
            .with_character('#')
            .with_foreground(Rgb24::new(0, 63, 63))
            .with_background(Rgb24::new(63, 127, 127)),
        Tile::Npc(NpcType::Orc) => ViewCell::new()
            .with_character('o')
            .with_bold(true)
            .with_foreground(colours::ORC),
        Tile::Npc(NpcType::Troll) => ViewCell::new()
            .with_character('T')
            .with_bold(true)
            .with_foreground(colours::TROLL),
        Tile::NpcCorpse(NpcType::Orc) => ViewCell::new()
            .with_character('%')
            .with_bold(true)
            .with_foreground(colours::ORC),
        Tile::NpcCorpse(NpcType::Troll) => ViewCell::new()
            .with_character('%')
            .with_bold(true)
            .with_foreground(colours::TROLL),
        Tile::Item(ItemType::HealthPotion) => ViewCell::new()
            .with_character('!')
            .with_foreground(colours::HEALTH_POTION),
    }
}

fn previously_visible_view_cell_of_tile(tile: Tile) -> ViewCell {
    match tile {
        Tile::Floor => ViewCell::new()
            .with_character('.')
            .with_foreground(Rgb24::new_grey(63))
            .with_background(Rgb24::new_grey(0)),
        Tile::Wall => ViewCell::new()
            .with_character('#')
            .with_foreground(Rgb24::new_grey(63))
            .with_background(Rgb24::new_grey(0)),
        _ => ViewCell::new(),
    }
}

#[derive(Default)]
struct GameView {}

impl<'a> View<&'a GameState> for GameView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        game_state: &'a GameState,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        for entity_to_render in game_state.entities_to_render() {
            let view_cell = match entity_to_render.visibility {
                CellVisibility::Currently => {
                    currently_visible_view_cell_of_tile(entity_to_render.tile)
                }
                CellVisibility::Previously => {
                    previously_visible_view_cell_of_tile(entity_to_render.tile)
                }
                CellVisibility::Never => ViewCell::new(),
            };
            let depth = match entity_to_render.location.layer {
                None => -1,
                Some(Layer::Floor) => 0,
                Some(Layer::Feature) => 1,
                Some(Layer::Object) => 2,
                Some(Layer::Character) => 3,
            };
            frame.set_cell_relative(entity_to_render.location.coord, depth, view_cell, context);
        }
    }
}

fn use_item() -> impl EventRoutine<Return = (), Data = AppData, View = AppView, Event = CommonEvent>
{
    make_either!(Ei = A | B);
    Loop::new(|| {
        inventory_slot_menu("Use Item").and_then(|result| match result {
            Err(menu::Escape) => Ei::A(Value::new(Some(()))),
            Ok(entry) => Ei::B(SideEffect::new_with_view(
                move |data: &mut AppData, _: &_| {
                    if data.game_state.maybe_player_use_item(entry.index).is_ok() {
                        Some(())
                    } else {
                        None
                    }
                },
            )),
        })
    })
}

fn drop_item() -> impl EventRoutine<Return = (), Data = AppData, View = AppView, Event = CommonEvent>
{
    make_either!(Ei = A | B);
    Loop::new(|| {
        inventory_slot_menu("Drop Item").and_then(|result| match result {
            Err(menu::Escape) => Ei::A(Value::new(Some(()))),
            Ok(entry) => Ei::B(SideEffect::new_with_view(
                move |data: &mut AppData, _: &_| {
                    if data.game_state.maybe_player_drop_item(entry.index).is_ok() {
                        Some(())
                    } else {
                        None
                    }
                },
            )),
        })
    })
}

fn game_loop() -> impl EventRoutine<Return = (), Data = AppData, View = AppView, Event = CommonEvent>
{
    make_either!(Ei = A | B | C | D);
    Loop::new(|| {
        GameEventRoutine.and_then(|game_return| match game_return {
            GameReturn::Exit => Ei::A(Value::new(Some(()))),
            GameReturn::GameOver => Ei::B(Value::new(Some(()))),
            GameReturn::UseItem => Ei::C(use_item().map(|_| None)),
            GameReturn::DropItem => Ei::D(drop_item().map(|_| None)),
        })
    })
    .return_on_exit(|_| ())
}

pub fn app(
    screen_size: Size,
    rng_seed: u64,
    visibility_algorithm: VisibilityAlgorithm,
) -> impl ChargridApp {
    let data = AppData::new(screen_size, rng_seed, visibility_algorithm);
    let view = AppView::new(screen_size);
    game_loop().app_one_shot_ignore_return(data, view)
}
