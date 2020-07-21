use crate::world::{ItemType, NpcType};
use grid_2d::{Coord, Grid, Size};
use rand::{seq::IteratorRandom, seq::SliceRandom, Rng};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TerrainTile {
    Player,
    Floor,
    Wall,
    Npc(NpcType),
    Item(ItemType),
}

// A rectangular area of the map
struct Room {
    top_left: Coord,
    size: Size,
}

impl Room {
    // Returns a randomly sized room at a random position within `bounds`
    fn choose<R: Rng>(bounds: Size, rng: &mut R) -> Self {
        let width = rng.gen_range(5..11);
        let height = rng.gen_range(5..9);
        let size = Size::new(width, height);
        let top_left_bounds = bounds - size;
        let left = rng.gen_range(0..top_left_bounds.width());
        let top = rng.gen_range(0..top_left_bounds.height());
        let top_left = Coord::new(left as i32, top as i32);
        Self { top_left, size }
    }

    // Returns a coord at the centre of the room, rounding down
    fn centre(&self) -> Coord {
        self.top_left + self.size.to_coord().unwrap() / 2
    }

    // Returns an iterator over all the coordinates in the room in row major order
    fn coords<'a>(&'a self) -> impl 'a + Iterator<Item = Coord> {
        self.size
            .coord_iter_row_major()
            .map(move |coord| self.top_left + coord)
    }

    // Returns true if and only if each cell of `grid` overlapping this room is `None`
    fn only_intersects_empty(&self, grid: &Grid<Option<TerrainTile>>) -> bool {
        self.coords().all(|coord| grid.get_checked(coord).is_none())
    }

    // Updates `grid`, setting each cell overlapping this room to `Some(TerrainTile::Floor)`.
    // The top and left sides of the room are set to `Some(TerrainTile::Wall)` instead.
    // This prevents a pair of rooms being placed immediately adjacent to one another.
    fn carve_out(&self, grid: &mut Grid<Option<TerrainTile>>) {
        for coord in self.coords() {
            let cell = grid.get_checked_mut(coord);
            if coord.x == self.top_left.x || coord.y == self.top_left.y {
                *cell = Some(TerrainTile::Wall);
            } else {
                *cell = Some(TerrainTile::Floor);
            }
        }
    }

    // Place `n` randomly chosen NPCs at random positions within the room
    fn place_npcs<R: Rng>(&self, n: usize, grid: &mut Grid<Option<TerrainTile>>, rng: &mut R) {
        for coord in self
            .coords()
            .filter(|&coord| grid.get_checked(coord).unwrap() == TerrainTile::Floor)
            .choose_multiple(rng, n)
        {
            let npc_type = if rng.gen_range(0..100) < 80 {
                NpcType::Orc
            } else {
                NpcType::Troll
            };
            *grid.get_checked_mut(coord) = Some(TerrainTile::Npc(npc_type));
        }
    }

    // Place `n` items at random positions within the room
    fn place_items<R: Rng>(&self, n: usize, grid: &mut Grid<Option<TerrainTile>>, rng: &mut R) {
        for coord in self
            .coords()
            .filter(|&coord| grid.get_checked(coord).unwrap() == TerrainTile::Floor)
            .choose_multiple(rng, n)
        {
            let item = match rng.gen_range(0..100) {
                0..=100 => ItemType::FireballScroll,
                _ => ItemType::HealthPotion,
            };
            *grid.get_checked_mut(coord) = Some(TerrainTile::Item(item));
        }
    }
}

// carve out an L-shaped corridor between a pair of coordinates
fn carve_corridor(start: Coord, end: Coord, grid: &mut Grid<Option<TerrainTile>>) {
    for i in start.x.min(end.x)..=start.x.max(end.x) {
        let cell = grid.get_checked_mut(Coord { x: i, ..start });
        if *cell == None || *cell == Some(TerrainTile::Wall) {
            *cell = Some(TerrainTile::Floor);
        }
    }
    for i in start.y.min(end.y)..start.y.max(end.y) {
        let cell = grid.get_checked_mut(Coord { y: i, ..end });
        if *cell == None || *cell == Some(TerrainTile::Wall) {
            *cell = Some(TerrainTile::Floor);
        }
    }
}

pub fn generate_dungeon<R: Rng>(size: Size, rng: &mut R) -> Grid<TerrainTile> {
    let mut grid = Grid::new_copy(size, None);
    let mut room_centres = Vec::new();

    const NPCS_PER_ROOM_DISTRIBUTION: &[usize] = &[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4];
    const ITEMS_PER_ROOM_DISTRIBUTION: &[usize] = &[0, 0, 1, 1, 1, 1, 1, 2, 2];

    // Attempt to add a room a constant number of times
    const NUM_ATTEMPTS: usize = 100;
    for _ in 0..NUM_ATTEMPTS {
        // Make a random room
        let room = Room::choose(size, rng);

        // Carve out the room unless it overlaps with an existing room
        if room.only_intersects_empty(&grid) {
            room.carve_out(&mut grid);

            let room_centre = room.centre();

            // Add the player to the centre of the room if it's the first room
            if room_centres.is_empty() {
                *grid.get_checked_mut(room_centre) = Some(TerrainTile::Player);
            }

            // Build up a list of all room centres for use in constructing corridors
            room_centres.push(room_centre);

            // Add npcs to the room
            let &num_npcs = NPCS_PER_ROOM_DISTRIBUTION.choose(rng).unwrap();
            room.place_npcs(num_npcs, &mut grid, rng);

            // Add items to the room
            let &num_items = ITEMS_PER_ROOM_DISTRIBUTION.choose(rng).unwrap();
            room.place_items(num_items, &mut grid, rng);
        }
    }

    // Add corridors connecting every adjacent pair of room centres
    for window in room_centres.windows(2) {
        carve_corridor(window[0], window[1], &mut grid);
    }

    grid.map(|t| t.unwrap_or(TerrainTile::Wall))
}
