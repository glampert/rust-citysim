use std::time::{self};
use rand::SeedableRng;
use rand_pcg::Pcg64;

use crate::{
    tile::{
        map::{Tile, TileMap, TileMapLayerKind},
        sets::{TileDef, TileKind, TileSets}
    },
    utils::{
        coords::{Cell, CellRange},
        hash::StringHash,
        UnsafeWeakRef
    }
};

use super::{
    building::{
        Building,
        BuildingKind
    },
    sim::world::{
        World
    }
};

pub mod resources;
pub mod world;

// ----------------------------------------------
// RandomGenerator
// ----------------------------------------------

const DEFAULT_RANDOM_SEED: u64 = 0xCAFE0CAFE0CAFE03;
pub type RandomGenerator = Pcg64;

// ----------------------------------------------
// Simulation
// ----------------------------------------------

const DEFAULT_SIM_UPDATE_FREQUENCY_SECS: f32 = 0.5;

pub struct Simulation {
    update_timer: UpdateTimer,
    rng: RandomGenerator,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            update_timer: UpdateTimer::new(DEFAULT_SIM_UPDATE_FREQUENCY_SECS),
            rng: RandomGenerator::seed_from_u64(DEFAULT_RANDOM_SEED),
        }
    }

    pub fn update<'tile_map, 'tile_sets>(&mut self,
                                         world: &mut World,
                                         tile_map: &'tile_map mut TileMap<'tile_sets>,
                                         tile_sets: &'tile_sets TileSets,
                                         delta_time: time::Duration) {

        // Fixed step update.
        let world_update_delta_time_secs = self.update_timer.time_since_last_secs();

        if self.update_timer.tick(delta_time.as_secs_f32()).should_update() {
            let mut query = Query::new(&mut self.rng, world, tile_map, tile_sets);
            world.update(&mut query, world_update_delta_time_secs);
        }
    }
}

// ----------------------------------------------
// UpdateTimer
// ----------------------------------------------

pub struct UpdateTimer {
    update_frequency_secs: f32,
    time_since_last_update_secs: f32,
}

#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum UpdateTimerResult {
    DoNotUpdate,
    ShouldUpdate,
}

impl UpdateTimerResult {
    #[inline]
    pub fn should_update(self) -> bool {
        self == UpdateTimerResult::ShouldUpdate
    }
}

impl UpdateTimer {
    #[inline]
    pub fn new(update_frequency_secs: f32) -> Self {
        Self {
            update_frequency_secs: update_frequency_secs,
            time_since_last_update_secs: 0.0,
        }
    }

    #[inline]
    pub fn tick(&mut self, delta_time_secs: f32) -> UpdateTimerResult {
        if self.time_since_last_update_secs >= self.update_frequency_secs {
            // Reset the clock.
            self.time_since_last_update_secs = 0.0;
            UpdateTimerResult::ShouldUpdate
        } else {
            // Advance the clock.
            self.time_since_last_update_secs += delta_time_secs;
            UpdateTimerResult::DoNotUpdate
        }
    }

    #[inline]
    pub fn frequency_secs(&self) -> f32 {
        self.update_frequency_secs
    }

    #[inline]
    pub fn time_since_last_secs(&self) -> f32 {
        self.time_since_last_update_secs
    }
}

// ----------------------------------------------
// Query
// ----------------------------------------------

pub struct Query<'config, 'sim, 'tile_map, 'tile_sets> {
    pub rng: &'sim mut RandomGenerator,
    pub tile_map: &'tile_map mut TileMap<'tile_sets>,
    pub tile_sets: &'tile_sets TileSets,

    // SAFETY: Queries are local variables in the Simulation::update() stack, so none
    // of the references stored here will persist or leak outside the call stack.
    // The reason we store this as a weak reference is because we cannot take another
    // reference to the world while we are also invoking update() on it, however,
    // a reference is required in some cases to look up other buildings.
    world: UnsafeWeakRef<World<'config>>,
}

impl<'config, 'sim, 'tile_map, 'tile_sets> Query<'config, 'sim, 'tile_map, 'tile_sets> {
    fn new(rng: &'sim mut RandomGenerator,
           world: &mut World<'config>,
           tile_map: &'tile_map mut TileMap<'tile_sets>,
           tile_sets: &'tile_sets TileSets) -> Self {
        Self {
            rng: rng,
            tile_map: tile_map,
            tile_sets: tile_sets,
            world: UnsafeWeakRef::new(world),
        }
    }

    #[inline]
    pub fn find_tile_def(&self,
                         layer: TileMapLayerKind,
                         category_name_hash: StringHash,
                         tile_def_name_hash: StringHash) -> Option<&'tile_sets TileDef> {

        self.tile_sets.find_tile_def_by_hash(layer, category_name_hash, tile_def_name_hash)
    }

    #[inline]
    pub fn find_tile(&self,
                     cell: Cell,
                     layer: TileMapLayerKind,
                     tile_kinds: TileKind) -> Option<&Tile<'tile_sets>> {

        self.tile_map.find_tile(cell, layer, tile_kinds)
    }

    #[inline]
    pub fn find_tile_mut(&mut self,
                         cell: Cell,
                         layer: TileMapLayerKind,
                         tile_kinds: TileKind) -> Option<&mut Tile<'tile_sets>> {

        self.tile_map.find_tile_mut(cell, layer, tile_kinds)
    }

    pub fn is_near_building(&self,
                            start_cells: CellRange,
                            kind: BuildingKind,
                            radius_in_cells: i32) -> bool {

        let search_range = Self::calc_search_range(start_cells, radius_in_cells);

        for search_cell in &search_range {
            if let Some(search_tile) =
                self.tile_map.find_tile(search_cell, TileMapLayerKind::Objects, TileKind::Building) {
                let game_state = search_tile.game_state_handle();
                if game_state.is_valid() {
                    let building_kind = BuildingKind::from_game_state_handle(game_state);
                    if building_kind == kind {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn find_nearest_building(&mut self,
                                 start_cells: CellRange,
                                 kind: BuildingKind,
                                 radius_in_cells: i32) -> Option<&mut Building<'config>> {

        let search_range = Self::calc_search_range(start_cells, radius_in_cells);

        for search_cell in &search_range {
            if let Some(search_tile) =
                self.tile_map.find_tile(search_cell, TileMapLayerKind::Objects, TileKind::Building) {
                let game_state = search_tile.game_state_handle();
                if game_state.is_valid() {
                    let building_kind = BuildingKind::from_game_state_handle(game_state);
                    if building_kind == kind {
                        return self.world.find_building_for_tile_mut(search_tile);
                    }
                }
            }
        }

        None
    }

    #[inline]
    fn calc_search_range(start_cells: CellRange, radius_in_cells: i32) -> CellRange {
        debug_assert!(start_cells.is_valid());
        debug_assert!(radius_in_cells > 0);

        let start_x = start_cells.start.x - radius_in_cells;
        let start_y = start_cells.start.y - radius_in_cells;
        let end_x   = start_cells.end.x   + radius_in_cells;
        let end_y   = start_cells.end.y   + radius_in_cells;
        CellRange::new(Cell::new(start_x, start_y), Cell::new(end_x, end_y))
    }
}
