use crate::world::World;
use coord_2d::{Coord, Size};
use direction::CardinalDirection;
use entity_table::Entity;
use grid_search_cardinal::{
    distance_map::{
        DistanceMap, PopulateContext as DistanceMapPopulateContext,
        SearchContext as DistanceMapSearchContext,
    },
    CanEnter,
};
use line_2d::LineSegment;
use shadowcast::{vision_distance, VisionDistance};

pub struct BehaviourContext {
    distance_map_to_player: DistanceMap,
    distance_map_populate_context: DistanceMapPopulateContext,
    distance_map_search_context: DistanceMapSearchContext,
}

impl BehaviourContext {
    pub fn new(size: Size) -> Self {
        Self {
            distance_map_to_player: DistanceMap::new(size),
            distance_map_populate_context: DistanceMapPopulateContext::default(),
            distance_map_search_context: DistanceMapSearchContext::new(size),
        }
    }

    pub fn update(&mut self, player: Entity, world: &World) {
        struct NpcCanEnterIgnoringOtherNpcs<'a> {
            world: &'a World,
        }
        impl<'a> CanEnter for NpcCanEnterIgnoringOtherNpcs<'a> {
            fn can_enter(&self, coord: Coord) -> bool {
                self.world.can_npc_enter_ignoring_other_npcs(coord)
            }
        }
        let player_coord = world.entity_coord(player).expect("player has no coord");
        const MAX_APPROACH_DISTANCE: u32 = 20;
        self.distance_map_populate_context.add(player_coord);
        self.distance_map_populate_context.populate_approach(
            &NpcCanEnterIgnoringOtherNpcs { world },
            MAX_APPROACH_DISTANCE,
            &mut self.distance_map_to_player,
        );
    }
}

pub enum NpcAction {
    Wait,
    Move(CardinalDirection),
}

pub struct Agent {}

fn npc_has_line_of_sight(src: Coord, dst: Coord, world: &World) -> bool {
    const NPC_VISION_DISTANCE_SQUARED: u32 = 100;
    const NPC_VISION_DISTANCE: vision_distance::Circle =
        vision_distance::Circle::new_squared(NPC_VISION_DISTANCE_SQUARED);
    if src == dst {
        return true;
    }
    for coord in LineSegment::new(src, dst).iter() {
        let src_to_coord = coord - src;
        if !NPC_VISION_DISTANCE.in_range(src_to_coord) {
            return false;
        }
        if !world.can_npc_see_through_cell(coord) {
            return false;
        }
    }
    true
}

impl Agent {
    pub fn new() -> Self {
        Self {}
    }

    pub fn act(
        &mut self,
        entity: Entity,
        player: Entity,
        world: &World,
        behaviour_context: &mut BehaviourContext,
    ) -> NpcAction {
        struct NpcCanEnter<'a> {
            world: &'a World,
        }
        impl<'a> CanEnter for NpcCanEnter<'a> {
            fn can_enter(&self, coord: Coord) -> bool {
                self.world.can_npc_enter(coord)
            }
        }
        let npc_coord = world.entity_coord(entity).expect("npc has no coord");
        let player_coord = world.entity_coord(player).expect("player has no coord");
        if !npc_has_line_of_sight(npc_coord, player_coord, world) {
            return NpcAction::Wait;
        }
        const SEARCH_DISTANCE: u32 = 5;
        match behaviour_context.distance_map_search_context.search_first(
            &NpcCanEnter { world },
            npc_coord,
            SEARCH_DISTANCE,
            &behaviour_context.distance_map_to_player,
        ) {
            None => NpcAction::Wait,
            Some(direction) => NpcAction::Move(direction),
        }
    }
}
