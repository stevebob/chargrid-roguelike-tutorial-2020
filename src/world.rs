use crate::behaviour::Agent;
use crate::terrain::{self, TerrainTile};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use entity_table::{ComponentTable, Entity, EntityAllocator};
use rand::Rng;

#[derive(Clone, Copy, Debug)]
pub struct HitPoints {
    pub current: u32,
    pub max: u32,
}

impl HitPoints {
    fn new_full(max: u32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcType {
    Orc,
    Troll,
}

impl NpcType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Orc => "orc",
            Self::Troll => "troll",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Tile {
    Player,
    PlayerCorpse,
    Floor,
    Wall,
    Npc(NpcType),
    NpcCorpse(NpcType),
}

entity_table::declare_entity_module! {
    components {
        tile: Tile,
        npc_type: NpcType,
        hit_points: HitPoints,
    }
}

use components::Components;

spatial_table::declare_layers_module! {
    layers {
        floor: Floor,
        character: Character,
        corpse: Corpse,
        feature: Feature,
    }
}

pub use layers::Layer;
type SpatialTable = spatial_table::SpatialTable<layers::Layers>;
pub type Location = spatial_table::Location<Layer>;

pub struct World {
    pub entity_allocator: EntityAllocator,
    pub components: Components,
    pub spatial_table: SpatialTable,
}

pub struct Populate {
    pub player_entity: Entity,
    pub ai_state: ComponentTable<Agent>,
}

impl World {
    pub fn new(size: Size) -> Self {
        let entity_allocator = EntityAllocator::default();
        let components = Components::default();
        let spatial_table = SpatialTable::new(size);
        Self {
            entity_allocator,
            components,
            spatial_table,
        }
    }
    fn spawn_wall(&mut self, coord: Coord) {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord,
                    layer: Some(Layer::Feature),
                },
            )
            .unwrap();
        self.components.tile.insert(entity, Tile::Wall);
    }
    fn spawn_floor(&mut self, coord: Coord) {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord,
                    layer: Some(Layer::Floor),
                },
            )
            .unwrap();
        self.components.tile.insert(entity, Tile::Floor);
    }
    fn spawn_player(&mut self, coord: Coord) -> Entity {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord,
                    layer: Some(Layer::Character),
                },
            )
            .unwrap();
        self.components.tile.insert(entity, Tile::Player);
        self.components
            .hit_points
            .insert(entity, HitPoints::new_full(20));
        entity
    }
    fn spawn_npc(&mut self, coord: Coord, npc_type: NpcType) -> Entity {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord,
                    layer: Some(Layer::Character),
                },
            )
            .unwrap();
        self.components.tile.insert(entity, Tile::Npc(npc_type));
        self.components.npc_type.insert(entity, npc_type);
        let hit_points = match npc_type {
            NpcType::Orc => HitPoints::new_full(2),
            NpcType::Troll => HitPoints::new_full(6),
        };
        self.components.hit_points.insert(entity, hit_points);
        entity
    }
    pub fn populate<R: Rng>(&mut self, rng: &mut R) -> Populate {
        let terrain = terrain::generate_dungeon(self.spatial_table.grid_size(), rng);
        let mut player_entity = None;
        let mut ai_state = ComponentTable::default();
        for (coord, &terrain_tile) in terrain.enumerate() {
            match terrain_tile {
                TerrainTile::Player => {
                    self.spawn_floor(coord);
                    player_entity = Some(self.spawn_player(coord));
                }
                TerrainTile::Floor => self.spawn_floor(coord),
                TerrainTile::Wall => {
                    self.spawn_floor(coord);
                    self.spawn_wall(coord);
                }
                TerrainTile::Npc(npc_type) => {
                    let entity = self.spawn_npc(coord, npc_type);
                    self.spawn_floor(coord);
                    ai_state.insert(entity, Agent::new());
                }
            }
        }
        Populate {
            player_entity: player_entity.unwrap(),
            ai_state,
        }
    }
    pub fn maybe_move_character(&mut self, character_entity: Entity, direction: CardinalDirection) {
        let character_coord = self
            .spatial_table
            .coord_of(character_entity)
            .expect("character has no coord");
        let new_character_coord = character_coord + direction.coord();
        if new_character_coord.is_valid(self.spatial_table.grid_size()) {
            let dest_layers = self.spatial_table.layers_at_checked(new_character_coord);
            if let Some(dest_character_entity) = dest_layers.character {
                let character_is_npc = self.components.npc_type.contains(character_entity);
                let dest_character_is_npc =
                    self.components.npc_type.contains(dest_character_entity);
                if character_is_npc != dest_character_is_npc {
                    self.character_bump_attack(dest_character_entity);
                }
            } else if dest_layers.feature.is_none() {
                self.spatial_table
                    .update_coord(character_entity, new_character_coord)
                    .unwrap();
            }
        }
    }
    fn character_bump_attack(&mut self, victim: Entity) {
        const DAMAGE: u32 = 1;
        if let Some(hit_points) = self.components.hit_points.get_mut(victim) {
            hit_points.current = hit_points.current.saturating_sub(DAMAGE);
            if hit_points.current == 0 {
                self.character_die(victim);
            }
        }
    }
    fn character_die(&mut self, entity: Entity) {
        if let Some(occpied_by_entity) = self
            .spatial_table
            .update_layer(entity, Layer::Corpse)
            .err()
            .map(|e| e.unwrap_occupied_by())
        {
            // If a character dies on a cell which contains a corpse, remove the existing corpse
            // from existence and replace it with the character's corpse.
            self.remove_entity(occpied_by_entity);
            self.spatial_table
                .update_layer(entity, Layer::Corpse)
                .unwrap();
        }
        let current_tile = self.components.tile.get(entity).unwrap();
        let corpse_tile = match current_tile {
            Tile::Player => Tile::PlayerCorpse,
            Tile::Npc(npc_type) => Tile::NpcCorpse(*npc_type),
            other => panic!("unexpected tile on character {:?}", other),
        };
        self.components.tile.insert(entity, corpse_tile);
    }
    pub fn is_living_character(&self, entity: Entity) -> bool {
        self.spatial_table.layer_of(entity) == Some(Layer::Character)
    }
    pub fn remove_entity(&mut self, entity: Entity) {
        self.components.remove_entity(entity);
        self.spatial_table.remove(entity);
        self.entity_allocator.free(entity);
    }
    pub fn size(&self) -> Size {
        self.spatial_table.grid_size()
    }
    pub fn opacity_at(&self, coord: Coord) -> u8 {
        if self
            .spatial_table
            .layers_at_checked(coord)
            .feature
            .is_some()
        {
            255
        } else {
            0
        }
    }
    pub fn entity_coord(&self, entity: Entity) -> Option<Coord> {
        self.spatial_table.coord_of(entity)
    }
    pub fn can_npc_enter_ignoring_other_npcs(&self, coord: Coord) -> bool {
        self.spatial_table
            .layers_at(coord)
            .map(|layers| layers.feature.is_none())
            .unwrap_or(false)
    }
    pub fn can_npc_enter(&self, coord: Coord) -> bool {
        self.spatial_table
            .layers_at(coord)
            .map(|layers| {
                let contains_npc = layers
                    .character
                    .map(|entity| self.components.npc_type.contains(entity))
                    .unwrap_or(false);
                let contains_feature = layers.feature.is_some();
                !(contains_npc || contains_feature)
            })
            .unwrap_or(false)
    }
    pub fn can_npc_see_through_cell(&self, coord: Coord) -> bool {
        self.spatial_table
            .layers_at(coord)
            .map(|layers| layers.feature.is_none())
            .unwrap_or(false)
    }
}
