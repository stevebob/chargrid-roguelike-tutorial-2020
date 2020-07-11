use crate::visibility::{CellVisibility, VisibilityAlgorithm, VisibilityGrid};
use crate::world::{Location, Populate, Tile, World};
use coord_2d::Size;
use direction::CardinalDirection;
use entity_table::ComponentTable;
use entity_table::Entity;
use rand::SeedableRng;
use rand_isaac::Isaac64Rng;

pub struct EntityToRender {
    pub tile: Tile,
    pub location: Location,
    pub visibility: CellVisibility,
}

pub struct GameState {
    world: World,
    player_entity: Entity,
    shadowcast_context: shadowcast::Context<u8>,
    visibility_grid: VisibilityGrid,
    ai_state: ComponentTable<()>,
}

impl GameState {
    pub fn new(
        screen_size: Size,
        rng_seed: u64,
        initial_visibility_algorithm: VisibilityAlgorithm,
    ) -> Self {
        let mut world = World::new(screen_size);
        let mut rng = Isaac64Rng::seed_from_u64(rng_seed);
        let Populate {
            player_entity,
            ai_state,
        } = world.populate(&mut rng);
        let shadowcast_context = shadowcast::Context::default();
        let visibility_grid = VisibilityGrid::new(screen_size);
        let mut game_state = Self {
            world,
            player_entity,
            shadowcast_context,
            visibility_grid,
            ai_state,
        };
        game_state.update_visibility(initial_visibility_algorithm);
        game_state
    }
    pub fn wait_player(&mut self) {
        self.ai_turn();
    }
    pub fn maybe_move_player(&mut self, direction: CardinalDirection) {
        self.world
            .maybe_move_character(self.player_entity, direction);
        self.ai_turn();
    }
    pub fn entities_to_render<'a>(&'a self) -> impl 'a + Iterator<Item = EntityToRender> {
        let tile_component = &self.world.components.tile;
        let spatial_table = &self.world.spatial_table;
        let visibility_grid = &self.visibility_grid;
        tile_component.iter().filter_map(move |(entity, &tile)| {
            let &location = spatial_table.location_of(entity)?;
            let visibility = visibility_grid.cell_visibility(location.coord);
            Some(EntityToRender {
                tile,
                location,
                visibility,
            })
        })
    }
    pub fn update_visibility(&mut self, visibility_algorithm: VisibilityAlgorithm) {
        let player_coord = self
            .world
            .spatial_table
            .coord_of(self.player_entity)
            .unwrap();
        self.visibility_grid.update(
            player_coord,
            &self.world,
            &mut self.shadowcast_context,
            visibility_algorithm,
        );
    }
    fn ai_turn(&mut self) {
        for (entity, ()) in self.ai_state.iter_mut() {
            let npc_type = self.world.npc_type(entity).unwrap();
            println!("The {} ponders its existence.", npc_type.name());
        }
    }
}
