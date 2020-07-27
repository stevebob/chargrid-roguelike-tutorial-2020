use crate::behaviour::Agent;
use crate::game::{ExamineCell, LogMessage};
use crate::terrain::{self, TerrainTile};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use entity_table::{ComponentTable, Entity, EntityAllocator};
use line_2d::CardinalStepIter;
use rand::Rng;

#[derive(Clone, Copy)]
pub enum ItemUsage {
    Immediate,
    Aim,
}

#[derive(Clone, Copy, Debug)]
pub enum ProjectileType {
    Fireball,
}

impl ProjectileType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Fireball => "fireball",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Inventory {
    slots: Vec<Option<Entity>>,
}

pub struct InventoryIsFull;

#[derive(Debug)]
pub struct InventorySlotIsEmpty;

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        let slots = vec![None; capacity];
        Self { slots }
    }
    pub fn slots(&self) -> &[Option<Entity>] {
        &self.slots
    }
    pub fn insert(&mut self, item: Entity) -> Result<(), InventoryIsFull> {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.is_none()) {
            *slot = Some(item);
            Ok(())
        } else {
            Err(InventoryIsFull)
        }
    }
    pub fn remove(&mut self, index: usize) -> Result<Entity, InventorySlotIsEmpty> {
        if let Some(slot) = self.slots.get_mut(index) {
            slot.take().ok_or(InventorySlotIsEmpty)
        } else {
            Err(InventorySlotIsEmpty)
        }
    }
    pub fn get(&self, index: usize) -> Result<Entity, InventorySlotIsEmpty> {
        self.slots
            .get(index)
            .cloned()
            .flatten()
            .ok_or(InventorySlotIsEmpty)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemType {
    HealthPotion,
    FireballScroll,
}

impl ItemType {
    pub fn name(self) -> &'static str {
        match self {
            Self::HealthPotion => "health potion",
            Self::FireballScroll => "fireball scroll",
        }
    }
}

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
    Item(ItemType),
    Projectile(ProjectileType),
}

entity_table::declare_entity_module! {
    components {
        tile: Tile,
        npc_type: NpcType,
        hit_points: HitPoints,
        item: ItemType,
        inventory: Inventory,
        trajectory: CardinalStepIter,
        projectile: ProjectileType,
    }
}

use components::Components;

spatial_table::declare_layers_module! {
    layers {
        floor: Floor,
        character: Character,
        object: Object,
        feature: Feature,
        projectile: Projectile,
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

struct VictimDies;

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
        self.components.inventory.insert(entity, Inventory::new(10));
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
    fn spawn_item(&mut self, coord: Coord, item_type: ItemType) {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord,
                    layer: Some(Layer::Object),
                },
            )
            .unwrap();
        self.components.tile.insert(entity, Tile::Item(item_type));
        self.components.item.insert(entity, item_type);
    }
    fn spawn_projectile(&mut self, from: Coord, to: Coord, projectile_type: ProjectileType) {
        let entity = self.entity_allocator.alloc();
        self.spatial_table
            .update(
                entity,
                Location {
                    coord: from,
                    layer: Some(Layer::Projectile),
                },
            )
            .unwrap();
        self.components
            .tile
            .insert(entity, Tile::Projectile(projectile_type));
        self.components.projectile.insert(entity, projectile_type);
        self.components
            .trajectory
            .insert(entity, CardinalStepIter::new(to - from));
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
                TerrainTile::Item(item_type) => {
                    self.spawn_item(coord, item_type);
                    self.spawn_floor(coord);
                }
            }
        }
        Populate {
            player_entity: player_entity.unwrap(),
            ai_state,
        }
    }
    fn write_combat_log_messages(
        attacker_is_player: bool,
        victim_dies: bool,
        npc_type: NpcType,
        message_log: &mut Vec<LogMessage>,
    ) {
        if attacker_is_player {
            if victim_dies {
                message_log.push(LogMessage::PlayerKillsNpc(npc_type));
            } else {
                message_log.push(LogMessage::PlayerAttacksNpc(npc_type));
            }
        } else {
            if victim_dies {
                message_log.push(LogMessage::NpcKillsPlayer(npc_type));
            } else {
                message_log.push(LogMessage::NpcAttacksPlayer(npc_type));
            }
        }
    }
    pub fn maybe_move_character(
        &mut self,
        character_entity: Entity,
        direction: CardinalDirection,
        message_log: &mut Vec<LogMessage>,
    ) {
        let character_coord = self
            .spatial_table
            .coord_of(character_entity)
            .expect("character has no coord");
        let new_character_coord = character_coord + direction.coord();
        if new_character_coord.is_valid(self.spatial_table.grid_size()) {
            let dest_layers = self.spatial_table.layers_at_checked(new_character_coord);
            if let Some(dest_character_entity) = dest_layers.character {
                let character_is_npc = self.components.npc_type.get(character_entity).cloned();
                let dest_character_is_npc =
                    self.components.npc_type.get(dest_character_entity).cloned();
                if character_is_npc.is_some() != dest_character_is_npc.is_some() {
                    let victim_dies = self.character_bump_attack(dest_character_entity).is_some();
                    let npc_type = character_is_npc.or(dest_character_is_npc).unwrap();
                    Self::write_combat_log_messages(
                        character_is_npc.is_none(),
                        victim_dies,
                        npc_type,
                        message_log,
                    );
                }
            } else if dest_layers.feature.is_none() {
                self.spatial_table
                    .update_coord(character_entity, new_character_coord)
                    .unwrap();
            }
        }
    }
    fn character_bump_attack(&mut self, victim: Entity) -> Option<VictimDies> {
        self.character_damage(victim, 1)
    }
    fn character_damage(&mut self, victim: Entity, damage: u32) -> Option<VictimDies> {
        if let Some(hit_points) = self.components.hit_points.get_mut(victim) {
            hit_points.current = hit_points.current.saturating_sub(damage);
            if hit_points.current == 0 {
                self.character_die(victim);
                return Some(VictimDies);
            }
        }
        None
    }
    fn character_die(&mut self, entity: Entity) {
        if let Some(occpied_by_entity) = self
            .spatial_table
            .update_layer(entity, Layer::Object)
            .err()
            .map(|e| e.unwrap_occupied_by())
        {
            // If a character dies on a cell which contains an object, remove the existing object
            // from existence and replace it with the character's corpse.
            self.remove_entity(occpied_by_entity);
            self.spatial_table
                .update_layer(entity, Layer::Object)
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
    pub fn maybe_get_item(
        &mut self,
        character: Entity,
        message_log: &mut Vec<LogMessage>,
    ) -> Result<(), ()> {
        let coord = self
            .spatial_table
            .coord_of(character)
            .expect("character has no coord");
        if let Some(object_entity) = self.spatial_table.layers_at_checked(coord).object {
            if let Some(&item_type) = self.components.item.get(object_entity) {
                // this assumes that the only character that can get items is the player
                let inventory = self
                    .components
                    .inventory
                    .get_mut(character)
                    .expect("character has no inventory");
                if inventory.insert(object_entity).is_ok() {
                    self.spatial_table.remove(object_entity);
                    message_log.push(LogMessage::PlayerGets(item_type));
                    return Ok(());
                } else {
                    message_log.push(LogMessage::PlayerInventoryIsFull);
                    return Err(());
                }
            }
        }
        message_log.push(LogMessage::NoItemUnderPlayer);
        Err(())
    }
    pub fn maybe_use_item(
        &mut self,
        character: Entity,
        inventory_index: usize,
        message_log: &mut Vec<LogMessage>,
    ) -> Result<ItemUsage, ()> {
        let inventory = self
            .components
            .inventory
            .get_mut(character)
            .expect("character has no inventory");
        let item = match inventory.get(inventory_index) {
            Ok(item) => item,
            Err(InventorySlotIsEmpty) => {
                message_log.push(LogMessage::NoItemInInventorySlot);
                return Err(());
            }
        };
        let &item_type = self
            .components
            .item
            .get(item)
            .expect("non-item in inventory");
        let usage = match item_type {
            ItemType::HealthPotion => {
                let mut hit_points = self
                    .components
                    .hit_points
                    .get_mut(character)
                    .expect("character has no hit points");
                const HEALTH_TO_HEAL: u32 = 5;
                hit_points.current = hit_points.max.min(hit_points.current + HEALTH_TO_HEAL);
                inventory.remove(inventory_index).unwrap();
                message_log.push(LogMessage::PlayerHeals);
                ItemUsage::Immediate
            }
            ItemType::FireballScroll => ItemUsage::Aim,
        };
        Ok(usage)
    }
    pub fn maybe_use_item_aim(
        &mut self,
        character: Entity,
        inventory_index: usize,
        target: Coord,
        message_log: &mut Vec<LogMessage>,
    ) -> Result<(), ()> {
        let character_coord = self.spatial_table.coord_of(character).unwrap();
        if character_coord == target {
            return Err(());
        }
        let inventory = self
            .components
            .inventory
            .get_mut(character)
            .expect("character has no inventory");
        let item_entity = inventory.remove(inventory_index).unwrap();
        let &item_type = self.components.item.get(item_entity).unwrap();
        match item_type {
            ItemType::HealthPotion => panic!("invalid item for aim"),
            ItemType::FireballScroll => {
                message_log.push(LogMessage::PlayerLaunchesProjectile(
                    ProjectileType::Fireball,
                ));
                self.spawn_projectile(character_coord, target, ProjectileType::Fireball);
            }
        }
        Ok(())
    }
    pub fn maybe_drop_item(
        &mut self,
        character: Entity,
        inventory_index: usize,
        message_log: &mut Vec<LogMessage>,
    ) -> Result<(), ()> {
        let coord = self
            .spatial_table
            .coord_of(character)
            .expect("character has no coord");
        if self.spatial_table.layers_at_checked(coord).object.is_some() {
            message_log.push(LogMessage::NoSpaceToDropItem);
            return Err(());
        }
        let inventory = self
            .components
            .inventory
            .get_mut(character)
            .expect("character has no inventory");
        let item = match inventory.remove(inventory_index) {
            Ok(item) => item,
            Err(InventorySlotIsEmpty) => {
                message_log.push(LogMessage::NoItemInInventorySlot);
                return Err(());
            }
        };
        self.spatial_table
            .update(
                item,
                Location {
                    coord,
                    layer: Some(Layer::Object),
                },
            )
            .unwrap();
        let &item_type = self
            .components
            .item
            .get(item)
            .expect("non-item in inventory");
        message_log.push(LogMessage::PlayerDrops(item_type));
        Ok(())
    }
    pub fn move_projectiles(&mut self, message_log: &mut Vec<LogMessage>) {
        let mut entities_to_remove = Vec::new();
        let mut fireball_hit = Vec::new();
        for (entity, trajectory) in self.components.trajectory.iter_mut() {
            if let Some(direction) = trajectory.next() {
                let current_coord = self.spatial_table.coord_of(entity).unwrap();
                let new_coord = current_coord + direction.coord();
                let dest_layers = self.spatial_table.layers_at_checked(new_coord);
                if dest_layers.feature.is_some() {
                    entities_to_remove.push(entity);
                } else if let Some(character) = dest_layers.character {
                    entities_to_remove.push(entity);
                    if let Some(&projectile_type) = self.components.projectile.get(entity) {
                        match projectile_type {
                            ProjectileType::Fireball => {
                                fireball_hit.push(character);
                            }
                        }
                    }
                }

                // ignore collisiosns of projectiles
                let _ = self.spatial_table.update_coord(entity, new_coord);
            } else {
                entities_to_remove.push(entity);
            }
        }
        for entity in entities_to_remove {
            self.remove_entity(entity);
        }
        for entity in fireball_hit {
            let maybe_npc = self.components.npc_type.get(entity).cloned();
            if let Some(VictimDies) = self.character_damage(entity, 2) {
                if let Some(npc) = maybe_npc {
                    message_log.push(LogMessage::NpcDies(npc));
                }
            }
        }
    }
    pub fn has_projectiles(&self) -> bool {
        !self.components.trajectory.is_empty()
    }
    pub fn inventory(&self, entity: Entity) -> Option<&Inventory> {
        self.components.inventory.get(entity)
    }
    pub fn item_type(&self, entity: Entity) -> Option<ItemType> {
        self.components.item.get(entity).cloned()
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
    pub fn hit_points(&self, entity: Entity) -> Option<HitPoints> {
        self.components.hit_points.get(entity).cloned()
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
    pub fn examine_cell(&self, coord: Coord) -> Option<ExamineCell> {
        let layers = self.spatial_table.layers_at(coord)?;
        layers
            .character
            .or_else(|| layers.object)
            .and_then(|entity| {
                self.components
                    .tile
                    .get(entity)
                    .and_then(|&tile| match tile {
                        Tile::Npc(npc_type) => Some(ExamineCell::Npc(npc_type)),
                        Tile::NpcCorpse(npc_type) => Some(ExamineCell::NpcCorpse(npc_type)),
                        Tile::Item(item_type) => Some(ExamineCell::Item(item_type)),
                        Tile::Player => Some(ExamineCell::Player),
                        _ => None,
                    })
            })
    }
}
