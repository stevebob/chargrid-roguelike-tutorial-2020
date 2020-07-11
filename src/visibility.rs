use crate::world::World;
use coord_2d::{Coord, Size};
use grid_2d::Grid;

#[derive(Clone, Copy, Debug)]
pub enum VisibilityAlgorithm {
    Shadowcast,
    Omniscient,
}

const VISION_DISTANCE_SQUARED: u32 = 100;
const VISION_DISTANCE: shadowcast::vision_distance::Circle =
    shadowcast::vision_distance::Circle::new_squared(VISION_DISTANCE_SQUARED);

struct Visibility;

impl shadowcast::InputGrid for Visibility {
    type Grid = World;
    type Opacity = u8;
    fn size(&self, world: &Self::Grid) -> Size {
        world.size()
    }
    fn get_opacity(&self, world: &Self::Grid, coord: Coord) -> Self::Opacity {
        world.opacity_at(coord)
    }
}

struct VisibilityCell {
    last_seen: u64,
}

impl Default for VisibilityCell {
    fn default() -> Self {
        Self { last_seen: 0 }
    }
}

pub enum CellVisibility {
    Currently,
    Previously,
    Never,
}

pub struct VisibilityGrid {
    grid: Grid<VisibilityCell>,
    count: u64,
}

impl VisibilityGrid {
    pub fn new(size: Size) -> Self {
        Self {
            grid: Grid::new_default(size),
            count: 1,
        }
    }
    pub fn cell_visibility(&self, coord: Coord) -> CellVisibility {
        if let Some(cell) = self.grid.get(coord) {
            if cell.last_seen == self.count {
                CellVisibility::Currently
            } else if cell.last_seen == 0 {
                CellVisibility::Never
            } else {
                CellVisibility::Previously
            }
        } else {
            CellVisibility::Never
        }
    }
    pub fn update(
        &mut self,
        player_coord: Coord,
        world: &World,
        shadowcast_context: &mut shadowcast::Context<u8>,
        algorithm: VisibilityAlgorithm,
    ) {
        self.count += 1;
        match algorithm {
            VisibilityAlgorithm::Omniscient => {
                for cell in self.grid.iter_mut() {
                    cell.last_seen = self.count;
                }
            }
            VisibilityAlgorithm::Shadowcast => {
                let count = self.count;
                let grid = &mut self.grid;
                shadowcast_context.for_each_visible(
                    player_coord,
                    &Visibility,
                    world,
                    VISION_DISTANCE,
                    255,
                    |coord, _visible_directions, _visibility| {
                        let cell = grid.get_checked_mut(coord);
                        cell.last_seen = count;
                    },
                );
            }
        }
    }
}
