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
    Stairs,
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
    fn place_npcs<R: Rng>(
        &self,
        n: usize,
        probability_distribution: &[(NpcType, u32)],
        grid: &mut Grid<Option<TerrainTile>>,
        rng: &mut R,
    ) {
        for coord in self
            .coords()
            .filter(|&coord| grid.get_checked(coord).unwrap() == TerrainTile::Floor)
            .choose_multiple(rng, n)
        {
            let &npc_type = choose_from_probability_distribution(probability_distribution, rng);
            *grid.get_checked_mut(coord) = Some(TerrainTile::Npc(npc_type));
        }
    }

    // Place `n` items at random positions within the room
    fn place_items<R: Rng>(
        &self,
        n: usize,
        probability_distribution: &[(ItemType, u32)],
        grid: &mut Grid<Option<TerrainTile>>,
        rng: &mut R,
    ) {
        for coord in self
            .coords()
            .filter(|&coord| grid.get_checked(coord).unwrap() == TerrainTile::Floor)
            .choose_multiple(rng, n)
        {
            let &item = choose_from_probability_distribution(probability_distribution, rng);
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

fn choose_from_probability_distribution<'a, T, R: Rng>(
    probability_distribution: &'a [(T, u32)],
    rng: &mut R,
) -> &'a T {
    let sum = probability_distribution.iter().map(|(_, p)| p).sum::<u32>();
    let mut choice = rng.gen_range(0..sum);
    for (value, probability) in probability_distribution.iter() {
        if let Some(remaining_choice) = choice.checked_sub(*probability) {
            choice = remaining_choice;
        } else {
            return value;
        }
    }
    unreachable!()
}

fn make_npc_probability_distribution(level: u32) -> Vec<(NpcType, u32)> {
    use NpcType::*;
    vec![(Orc, 20), (Troll, level)]
}

fn make_item_probability_distribution(level: u32) -> Vec<(ItemType, u32)> {
    use ItemType::*;
    let item_chance = match level {
        0..=1 => 5,
        2..=3 => 10,
        _ => 20,
    };
    vec![
        (HealthPotion, 200),
        (
            FireballScroll,
            match level {
                0..=1 => 10,
                2..=4 => 50,
                _ => 100,
            },
        ),
        (
            ConfusionScroll,
            match level {
                0..=1 => 10,
                2..=4 => 30,
                _ => 50,
            },
        ),
        (Sword, item_chance),
        (Staff, item_chance),
        (Armour, item_chance),
        (Robe, item_chance),
    ]
}

pub fn generate_dungeon<R: Rng>(size: Size, level: u32, rng: &mut R) -> Grid<TerrainTile> {
    let mut grid = Grid::new_copy(size, None);
    let mut room_centres = Vec::new();

    const NPCS_PER_ROOM_DISTRIBUTION: &[usize] = &[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4];
    const ITEMS_PER_ROOM_DISTRIBUTION: &[usize] = &[0, 0, 1, 1, 1, 1, 1, 2, 2];

    let npc_probability_distribution = make_npc_probability_distribution(level);
    let item_probability_distribution = make_item_probability_distribution(level);

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
            room.place_npcs(num_npcs, &npc_probability_distribution, &mut grid, rng);

            // Add items to the room
            let &num_items = ITEMS_PER_ROOM_DISTRIBUTION.choose(rng).unwrap();
            room.place_items(num_items, &item_probability_distribution, &mut grid, rng);
        }
    }

    // Add corridors connecting every adjacent pair of room centres
    for window in room_centres.windows(2) {
        carve_corridor(window[0], window[1], &mut grid);
    }

    // Add stairs to the centre of the last room placed
    *grid.get_checked_mut(*room_centres.last().unwrap()) = Some(TerrainTile::Stairs);

    grid.map(|t| t.unwrap_or(TerrainTile::Wall))
}
