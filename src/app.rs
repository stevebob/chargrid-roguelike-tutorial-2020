use crate::game::GameState;
use crate::ui::{UiData, UiView};
use crate::visibility::{CellVisibility, VisibilityAlgorithm};
use crate::world::{ItemType, Layer, NpcType, Tile};
use chargrid::{
    app::{App as ChargridApp, ControlFlow},
    decorator::{
        AlignView, Alignment, BorderStyle, BorderView, BoundView, FillBackgroundView, MinSizeView,
    },
    input::{keys, Input, KeyboardInput},
    menu::{
        self, MenuIndexFromScreenCoord, MenuInstanceBuilder, MenuInstanceChoose,
        MenuInstanceChooseOrEscape, MenuInstanceMouseTracker,
    },
    render::{ColModify, ColModifyMap, Frame, Style, View, ViewCell, ViewContext},
    text::{RichTextPart, RichTextViewSingleLine},
};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use rgb24::Rgb24;
use std::collections::HashMap;
use std::time::Duration;

const UI_NUM_ROWS: u32 = 5;

#[derive(Clone, Copy, Debug)]
struct InventorySlotMenuEntry {
    index: usize,
    key: char,
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

#[derive(Clone, Copy, Debug)]
enum AppStateMenu {
    UseItem,
    DropItem,
}

#[derive(Clone, Copy, Debug)]
enum AppState {
    Game,
    Menu(AppStateMenu),
}

struct AppData {
    game_state: GameState,
    visibility_algorithm: VisibilityAlgorithm,
    inventory_slot_menu: MenuInstanceChooseOrEscape<InventorySlotMenuEntry>,
    app_state: AppState,
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
            app_state: AppState::Game,
        }
    }
    fn handle_input(&mut self, input: Input, view: &AppView) -> Option<ControlFlow> {
        if !self.game_state.is_player_alive() {
            return None;
        }
        match self.app_state {
            AppState::Game => match input {
                Input::Keyboard(key) => match key {
                    KeyboardInput::Left => {
                        self.game_state.maybe_move_player(CardinalDirection::West)
                    }
                    KeyboardInput::Right => {
                        self.game_state.maybe_move_player(CardinalDirection::East)
                    }
                    KeyboardInput::Up => {
                        self.game_state.maybe_move_player(CardinalDirection::North)
                    }
                    KeyboardInput::Down => {
                        self.game_state.maybe_move_player(CardinalDirection::South)
                    }
                    KeyboardInput::Char(' ') => self.game_state.wait_player(),
                    KeyboardInput::Char('g') => self.game_state.maybe_player_get_item(),
                    KeyboardInput::Char('i') => {
                        self.app_state = AppState::Menu(AppStateMenu::UseItem)
                    }
                    KeyboardInput::Char('d') => {
                        self.app_state = AppState::Menu(AppStateMenu::DropItem)
                    }
                    keys::ESCAPE => return Some(ControlFlow::Exit),
                    _ => (),
                },
                _ => (),
            },
            AppState::Menu(menu) => match self
                .inventory_slot_menu
                .choose(&view.inventory_slot_menu_view, input)
            {
                None => (),
                Some(Err(menu::Escape)) => self.app_state = AppState::Game,
                Some(Ok(entry)) => match menu {
                    AppStateMenu::UseItem => {
                        if self.game_state.maybe_player_use_item(entry.index).is_ok() {
                            self.app_state = AppState::Game;
                        }
                    }
                    AppStateMenu::DropItem => {
                        if self.game_state.maybe_player_drop_item(entry.index).is_ok() {
                            self.app_state = AppState::Game;
                        }
                    }
                },
            },
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

impl<'a> View<&'a AppData> for AppView {
    fn view<F: Frame, C: ColModify>(
        &mut self,
        data: &'a AppData,
        context: ViewContext<C>,
        frame: &mut F,
    ) {
        fn col_modify_dim(num: u32, denom: u32) -> impl ColModify {
            ColModifyMap(move |col: Rgb24| col.saturating_scalar_mul_div(num, denom))
        }
        let game_col_modify = match data.app_state {
            AppState::Game => col_modify_dim(1, 1),
            AppState::Menu(menu) => {
                let title_text = match menu {
                    AppStateMenu::UseItem => "Use Item",
                    AppStateMenu::DropItem => "Drop Item",
                };
                BoundView {
                    size: data.game_state.size(),
                    view: AlignView {
                        alignment: Alignment::centre(),
                        view: FillBackgroundView {
                            rgb24: Rgb24::new_grey(0),
                            view: BorderView {
                                style: &BorderStyle {
                                    title: Some(title_text.to_string()),
                                    title_style: Style::new().with_foreground(Rgb24::new_grey(255)),
                                    ..Default::default()
                                },
                                view: MinSizeView {
                                    size: Size::new(12, 0),
                                    view: &mut self.inventory_slot_menu_view,
                                },
                            },
                        },
                    },
                }
                .view(data, context.add_depth(10), frame);
                col_modify_dim(1, 2)
            }
        };
        self.game_view.view(
            &data.game_state,
            context.compose_col_modify(game_col_modify),
            frame,
        );
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

pub struct App {
    data: AppData,
    view: AppView,
}

impl App {
    pub fn new(
        screen_size: Size,
        rng_seed: u64,
        visibility_algorithm: VisibilityAlgorithm,
    ) -> Self {
        Self {
            data: AppData::new(screen_size, rng_seed, visibility_algorithm),
            view: AppView::new(screen_size),
        }
    }
}

impl ChargridApp for App {
    fn on_input(&mut self, input: Input) -> Option<ControlFlow> {
        match input {
            Input::Keyboard(keys::ETX) => Some(ControlFlow::Exit),
            other => self.data.handle_input(other, &self.view),
        }
    }
    fn on_frame<F, C>(
        &mut self,
        _since_last_frame: Duration,
        view_context: ViewContext<C>,
        frame: &mut F,
    ) -> Option<ControlFlow>
    where
        F: Frame,
        C: ColModify,
    {
        self.view.view(&self.data, view_context, frame);
        None
    }
}
