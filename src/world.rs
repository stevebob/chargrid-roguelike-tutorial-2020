use crate::behaviour::Agent;
use crate::game::{ExamineCell, LevelUp, LogMessage};
use crate::terrain::{self, TerrainTile};
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use entity_table::{ComponentTable, Entity, EntityAllocator};
use line_2d::CardinalStepIter;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub struct EquippedInventoryIndices {
    pub worn: Option<usize>,
    pub held: Option<usize>,
}

pub struct CharacterData {
    entity_data: EntityData,
    inventory_entity_data: Vec<Option<EntityData>>,
}

#[derive(Clone, Copy)]
pub enum ItemUsage {
    Immediate,
    Aim,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProjectileType {
    Fireball { damage: u32 },
    Confusion { duration: u32 },
}

impl ProjectileType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Fireball { .. } => "fireball",
            Self::Confusion { .. } => "confusion spell",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    HealthPotion,
    FireballScroll,
    ConfusionScroll,
    Sword,
    Staff,
    Armour,
    Robe,
}

impl ItemType {
    pub fn name(self) -> &'static str {
        match self {
            Self::HealthPotion => "health potion",
            Self::FireballScroll => "fireball scroll",
            Self::ConfusionScroll => "confusion scroll",
            Self::Sword => "sword",
            Self::Staff => "staff",
            Self::Armour => "armour",
            Self::Robe => "robe",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct HitPoints {
    pub current: u32,
    pub max: u32,
}

impl HitPoints {
    fn new_full(max: u32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Tile {
    Player,
    PlayerCorpse,
    Floor,
    Wall,
    Npc(NpcType),
    NpcCorpse(NpcType),
    Item(ItemType),
    Projectile(ProjectileType),
    Stairs,
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
        confusion_countdown: u32,
        stairs: (),
        base_damage: i32,
        strength: i32,
        dexterity: i32,
        intelligence: i32,
        equipment_worn_inventory_index: usize,
        equipment_held_inventory_index: usize,
    }
}

use components::Components;
pub use components::EntityData;

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

#[derive(Serialize, Deserialize)]
pub struct World {
    pub entity_allocator: EntityAllocator,
    pub components: Components,
    pub spatial_table: SpatialTable,
}

pub struct Populate {
    pub player_entity: Entity,
    pub ai_state: ComponentTable<Agent>,
}

enum BumpAttackOutcome {
    Hit,
    Dodge,
    Kill,
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
    pub fn clear(&mut self) {
        self.entity_allocator.clear();
        self.components.clear();
        self.spatial_table.clear();
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
        self.components.base_damage.insert(entity, 1);
        self.components.strength.insert(entity, 1);
        self.components.dexterity.insert(entity, 1);
        self.components.intelligence.insert(entity, 1);
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
        self.components.base_damage.insert(entity, 1);
        let (strength, dexterity) = match npc_type {
            NpcType::Orc => (1, 1),
            NpcType::Troll => (2, 0),
        };
        self.components.strength.insert(entity, strength);
        self.components.dexterity.insert(entity, dexterity);
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
    fn spawn_stairs(&mut self, coord: Coord) {
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
        self.components.tile.insert(entity, Tile::Stairs);
        self.components.stairs.insert(entity, ());
    }
    pub fn populate<R: Rng>(&mut self, level: u32, rng: &mut R) -> Populate {
        let terrain = terrain::generate_dungeon(self.spatial_table.grid_size(), level, rng);
        let mut player_entity = None;
        let mut ai_state = ComponentTable::default();
        for (coord, &terrain_tile) in terrain.enumerate() {
            match terrain_tile {
                TerrainTile::Player => {
                    self.spawn_floor(coord);
                    player_entity = Some(self.spawn_player(coord));
                }
                TerrainTile::Floor => self.spawn_floor(coord),
                TerrainTile::Stairs => self.spawn_stairs(coord),
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
        outcome: BumpAttackOutcome,
        npc_type: NpcType,
        message_log: &mut Vec<LogMessage>,
    ) {
        if attacker_is_player {
            match outcome {
                BumpAttackOutcome::Kill => message_log.push(LogMessage::PlayerKillsNpc(npc_type)),
                BumpAttackOutcome::Hit => message_log.push(LogMessage::PlayerAttacksNpc(npc_type)),
                BumpAttackOutcome::Dodge => message_log.push(LogMessage::NpcDodges(npc_type)),
            }
        } else {
            match outcome {
                BumpAttackOutcome::Kill => message_log.push(LogMessage::NpcKillsPlayer(npc_type)),
                BumpAttackOutcome::Hit => message_log.push(LogMessage::NpcAttacksPlayer(npc_type)),
                BumpAttackOutcome::Dodge => message_log.push(LogMessage::PlayerDodges(npc_type)),
            }
        }
    }
    pub fn maybe_move_character<R: Rng>(
        &mut self,
        character_entity: Entity,
        direction: CardinalDirection,
        message_log: &mut Vec<LogMessage>,
        rng: &mut R,
    ) {
        let character_coord = self
            .spatial_table
            .coord_of(character_entity)
            .expect("character has no coord");
        let direction = if let Some(confusion_countdown) = self
            .components
            .confusion_countdown
            .get_mut(character_entity)
        {
            if *confusion_countdown == 0 {
                self.components.confusion_countdown.remove(character_entity);
                if let Some(&npc_type) = self.components.npc_type.get(character_entity) {
                    message_log.push(LogMessage::NpcIsNoLongerConfused(npc_type));
                }
            } else {
                *confusion_countdown -= 1;
            }
            rng.gen()
        } else {
            direction
        };
        let new_character_coord = character_coord + direction.coord();
        if new_character_coord.is_valid(self.spatial_table.grid_size()) {
            let dest_layers = self.spatial_table.layers_at_checked(new_character_coord);
            if let Some(dest_character_entity) = dest_layers.character {
                let character_is_npc = self.components.npc_type.get(character_entity).cloned();
                let dest_character_is_npc =
                    self.components.npc_type.get(dest_character_entity).cloned();
                if character_is_npc.is_some() != dest_character_is_npc.is_some() {
                    let outcome =
                        self.character_bump_attack(dest_character_entity, character_entity, rng);
                    let npc_type = character_is_npc.or(dest_character_is_npc).unwrap();
                    Self::write_combat_log_messages(
                        character_is_npc.is_none(),
                        outcome,
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
    fn character_bump_attack<R: Rng>(
        &mut self,
        victim: Entity,
        attacker: Entity,
        rng: &mut R,
    ) -> BumpAttackOutcome {
        let &attacker_base_damage = self.components.base_damage.get(attacker).unwrap();
        let &attacker_strength = self.components.strength.get(attacker).unwrap();
        let &victim_dexterity = self.components.dexterity.get(victim).unwrap();
        let gross_damage = attacker_base_damage + rng.gen_range(0..(attacker_strength + 1));
        let damage_reduction = rng.gen_range(0..(victim_dexterity + 1));
        let net_damage = gross_damage.saturating_sub(damage_reduction) as u32;
        if net_damage == 0 {
            BumpAttackOutcome::Dodge
        } else {
            if self.character_damage(victim, net_damage).is_some() {
                BumpAttackOutcome::Kill
            } else {
                BumpAttackOutcome::Hit
            }
        }
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
            ItemType::FireballScroll | ItemType::ConfusionScroll => ItemUsage::Aim,
            ItemType::Sword | ItemType::Staff => {
                self.components
                    .equipment_held_inventory_index
                    .insert(character, inventory_index);
                ItemUsage::Immediate
            }
            ItemType::Armour | ItemType::Robe => {
                self.components
                    .equipment_worn_inventory_index
                    .insert(character, inventory_index);
                ItemUsage::Immediate
            }
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
            ItemType::HealthPotion
            | ItemType::Sword
            | ItemType::Staff
            | ItemType::Armour
            | ItemType::Robe => panic!("invalid item for aim"),
            ItemType::FireballScroll => {
                let fireball = ProjectileType::Fireball {
                    damage: (*self.components.intelligence.get(character).unwrap()).max(0) as u32,
                };
                message_log.push(LogMessage::PlayerLaunchesProjectile(fireball));
                self.spawn_projectile(character_coord, target, fireball);
            }
            ItemType::ConfusionScroll => {
                let confusion = ProjectileType::Confusion {
                    duration: (*self.components.intelligence.get(character).unwrap()).max(0) as u32
                        * 3,
                };
                message_log.push(LogMessage::PlayerLaunchesProjectile(confusion));
                self.spawn_projectile(character_coord, target, confusion);
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
        if self
            .components
            .equipment_held_inventory_index
            .get(character)
            .cloned()
            == Some(inventory_index)
        {
            self.components
                .equipment_held_inventory_index
                .remove(character);
        }
        if self
            .components
            .equipment_worn_inventory_index
            .get(character)
            .cloned()
            == Some(inventory_index)
        {
            self.components
                .equipment_worn_inventory_index
                .remove(character);
        }
        message_log.push(LogMessage::PlayerDrops(item_type));
        Ok(())
    }
    pub fn move_projectiles(&mut self, message_log: &mut Vec<LogMessage>) {
        let mut entities_to_remove = Vec::new();
        let mut fireball_hit = Vec::new();
        let mut confusion_hit = Vec::new();
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
                            ProjectileType::Fireball { damage } => {
                                fireball_hit.push((character, damage));
                            }
                            ProjectileType::Confusion { duration } => {
                                confusion_hit.push((character, duration));
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
        for (entity, damage) in fireball_hit {
            let maybe_npc = self.components.npc_type.get(entity).cloned();
            if let Some(VictimDies) = self.character_damage(entity, damage) {
                if let Some(npc) = maybe_npc {
                    message_log.push(LogMessage::NpcDies(npc));
                }
            }
        }
        for (entity, duration) in confusion_hit {
            self.components.confusion_countdown.insert(entity, duration);
            if let Some(&npc_type) = self.components.npc_type.get(entity) {
                message_log.push(LogMessage::NpcBecomesConfused(npc_type));
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
    fn remove_entity_data(&mut self, entity: Entity) -> EntityData {
        self.entity_allocator.free(entity);
        self.spatial_table.remove(entity);
        self.components.remove_entity_data(entity)
    }
    pub fn remove_character(&mut self, entity: Entity) -> CharacterData {
        let mut entity_data = self.remove_entity_data(entity);
        // Remove the inventory from the character. An inventory contains entities referring data
        // in the current world. These data will also be removed here, and combined with the
        // `EntityData` of the character to form a `CharacterData`. When the `CharacterData` is
        // re-inserted into the world, the inventory item data will be inserted first, at which
        // point each item will be assigned a fresh entity. The character will get a brand new
        // inventory containing the new entities.
        let inventory_entity_data = entity_data
            .inventory
            .take()
            .expect("character missing inventory")
            .slots()
            .iter()
            .map(|maybe_slot| maybe_slot.map(|entity| self.remove_entity_data(entity)))
            .collect::<Vec<_>>();
        CharacterData {
            entity_data,
            inventory_entity_data,
        }
    }
    pub fn replace_character(
        &mut self,
        entity: Entity,
        CharacterData {
            mut entity_data,
            inventory_entity_data,
        }: CharacterData,
    ) {
        // Before inserting the character's data, create new entities to contain each item in the
        // character's inventory.
        let inventory_slots = inventory_entity_data
            .into_iter()
            .map(|maybe_entity_data| {
                maybe_entity_data.map(|entity_data| {
                    let entity = self.entity_allocator.alloc();
                    self.components.update_entity_data(entity, entity_data);
                    entity
                })
            })
            .collect::<Vec<_>>();
        // Make a new inventory containing the newly created entities, and add it to the character.
        entity_data.inventory = Some(Inventory {
            slots: inventory_slots,
        });
        self.components.update_entity_data(entity, entity_data);
    }
    pub fn coord_contains_stairs(&self, coord: Coord) -> bool {
        self.spatial_table
            .layers_at_checked(coord)
            .floor
            .map(|floor_entity| self.components.stairs.contains(floor_entity))
            .unwrap_or(false)
    }
    pub fn strength(&self, entity: Entity) -> Option<i32> {
        self.components.strength.get(entity).cloned()
    }
    pub fn dexterity(&self, entity: Entity) -> Option<i32> {
        self.components.dexterity.get(entity).cloned()
    }
    pub fn intelligence(&self, entity: Entity) -> Option<i32> {
        self.components.intelligence.get(entity).cloned()
    }
    pub fn level_up_character(&mut self, character_entity: Entity, level_up: LevelUp) {
        match level_up {
            LevelUp::Strength => {
                *self
                    .components
                    .strength
                    .get_mut(character_entity)
                    .expect("character lacks strength") += 1;
            }
            LevelUp::Dexterity => {
                *self
                    .components
                    .dexterity
                    .get_mut(character_entity)
                    .expect("character lacks dexterity") += 1;
            }
            LevelUp::Intelligence => {
                *self
                    .components
                    .intelligence
                    .get_mut(character_entity)
                    .expect("character lacks intelligence") += 1;
            }
            LevelUp::Health => {
                let hit_points = self
                    .components
                    .hit_points
                    .get_mut(character_entity)
                    .expect("character lacks hit points");
                const INCREASE: u32 = 5;
                hit_points.current += INCREASE;
                hit_points.max += INCREASE;
            }
        }
    }
    pub fn equipped_inventory_indices(&self, entity: Entity) -> EquippedInventoryIndices {
        let held = self
            .components
            .equipment_held_inventory_index
            .get(entity)
            .cloned();
        let worn = self
            .components
            .equipment_worn_inventory_index
            .get(entity)
            .cloned();
        EquippedInventoryIndices { held, worn }
    }
}
