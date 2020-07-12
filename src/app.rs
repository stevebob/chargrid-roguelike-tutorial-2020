use crate::game::GameState;
use crate::ui::{UiData, UiView};
use crate::visibility::{CellVisibility, VisibilityAlgorithm};
use crate::world::{Layer, NpcType, Tile};
use chargrid::{
    app::{App as ChargridApp, ControlFlow},
    input::{keys, Input, KeyboardInput},
    render::{ColModify, Frame, View, ViewCell, ViewContext},
};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use rgb24::Rgb24;
use std::time::Duration;

const UI_NUM_ROWS: u32 = 2;

struct AppData {
    game_state: GameState,
    visibility_algorithm: VisibilityAlgorithm,
}

impl AppData {
    fn new(screen_size: Size, rng_seed: u64, visibility_algorithm: VisibilityAlgorithm) -> Self {
        let game_area_size = screen_size.set_height(screen_size.height() - UI_NUM_ROWS);
        Self {
            game_state: GameState::new(game_area_size, rng_seed, visibility_algorithm),
            visibility_algorithm,
        }
    }
    fn handle_input(&mut self, input: Input) {
        if !self.game_state.is_player_alive() {
            return;
        }
        match input {
            Input::Keyboard(key) => match key {
                KeyboardInput::Left => self.game_state.maybe_move_player(CardinalDirection::West),
                KeyboardInput::Right => self.game_state.maybe_move_player(CardinalDirection::East),
                KeyboardInput::Up => self.game_state.maybe_move_player(CardinalDirection::North),
                KeyboardInput::Down => self.game_state.maybe_move_player(CardinalDirection::South),
                KeyboardInput::Char(' ') => self.game_state.wait_player(),
                _ => (),
            },
            _ => (),
        }
        self.game_state.update_visibility(self.visibility_algorithm);
    }
}

struct AppView {
    ui_y_offset: i32,
    game_view: GameView,
    ui_view: UiView,
}

impl AppView {
    fn new(screen_size: Size) -> Self {
        const UI_Y_PADDING: u32 = 1;
        let ui_y_offset = (screen_size.height() - UI_NUM_ROWS + UI_Y_PADDING) as i32;
        Self {
            ui_y_offset,
            game_view: GameView::default(),
            ui_view: UiView::default(),
        }
    }
}

mod colours {
    use rgb24::Rgb24;
    pub const PLAYER: Rgb24 = Rgb24::new_grey(255);
    pub const ORC: Rgb24 = Rgb24::new(0, 187, 0);
    pub const TROLL: Rgb24 = Rgb24::new(187, 0, 0);
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
                Some(Layer::Corpse) => 2,
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
        self.game_view.view(&data.game_state, context, frame);
        let player_hit_points = data.game_state.player_hit_points();
        self.ui_view.view(
            UiData { player_hit_points },
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
            Input::Keyboard(keys::ETX) | Input::Keyboard(keys::ESCAPE) => Some(ControlFlow::Exit),
            other => {
                self.data.handle_input(other);
                None
            }
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
