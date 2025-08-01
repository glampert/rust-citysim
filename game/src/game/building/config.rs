use crate::{
    utils::hash::{self, StringHash},
    tile::map::Tile,
    game::sim::resources::{
        ResourceKind,
        ResourceKinds,
        ServiceKinds
    }
};

use super::{
    Building,
    BuildingKind,
    BuildingArchetype,
    house::{
        HouseLevel,
        HouseConfig,
        HouseLevelConfig,
        HouseBuilding
    },
    producer::{
        ProducerConfig,
        ProducerBuilding
    },
    service::{
        ServiceConfig,
        ServiceBuilding
    },
    storage::{
        StorageConfig,
        StorageBuilding
    }
};

// ----------------------------------------------
// BuildingConfigs
// ----------------------------------------------

pub struct BuildingConfigs {
    // TODO: Temporary
    house_cfg: HouseConfig,
    house0: HouseLevelConfig,
    house1: HouseLevelConfig,
    house2: HouseLevelConfig,
    service_well_small: ServiceConfig,
    service_well_big: ServiceConfig,
    service_market: ServiceConfig,
    producer_rice_farm: ProducerConfig,
    producer_livestock_farm: ProducerConfig,
    storage_yard: StorageConfig,
    storage_granary: StorageConfig,
}

impl BuildingConfigs {
    // TODO: Load from config file.
    pub fn load() -> Self {
        Self {
            house_cfg: HouseConfig {
                // General configuration parameters for all house buildings & levels.
                stock_update_frequency_secs: 20.0,
                upgrade_update_frequency_secs: 10.0,
            },
            house0: HouseLevelConfig {
                name: "House Level 0".to_string(),
                tile_def_name: "house0".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("house0"),
                max_residents: 2,
                tax_generated: 0,
                services_required: ServiceKinds::none(),
                resources_required: ResourceKinds::none(),        
            },
            house1: HouseLevelConfig {
                name: "House Level 1".to_string(),
                tile_def_name: "house1".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("house1"),
                max_residents: 4,
                tax_generated: 1,
                // Any water source (small well OR big well) AND a market.
                services_required: ServiceKinds::with_slice(&[BuildingKind::WellSmall | BuildingKind::WellBig, BuildingKind::Market]),
                // Any 1 kind of food.
                resources_required: ResourceKinds::with_slice(&[ResourceKind::foods()]),
            },
            house2: HouseLevelConfig {
                name: "House Level 2".to_string(),
                tile_def_name: "house2".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("house2"),
                max_residents: 6,
                tax_generated: 2,
                services_required: ServiceKinds::with_slice(&[BuildingKind::WellBig, BuildingKind::Market]),
                // 2 kinds of food required: Rice AND Meat OR Fish.
                resources_required: ResourceKinds::with_slice(&[ResourceKind::Rice, ResourceKind::Meat | ResourceKind::Fish]),
            },
            service_well_small: ServiceConfig {
                name: "Well Small".to_string(),
                tile_def_name: "well_small".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("well_small"),
                min_workers: 0,
                max_workers: 1,
                stock_update_frequency_secs: 0.0,
                effect_radius: 3,
                resources_required: ResourceKinds::none(),
            },
            service_well_big: ServiceConfig {
                name: "Well Big".to_string(),
                tile_def_name: "well_big".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("well_big"),
                min_workers: 0,
                max_workers: 1,
                stock_update_frequency_secs: 0.0,
                effect_radius: 5,
                resources_required: ResourceKinds::none(),
            },
            service_market: ServiceConfig {
                name: "Market".to_string(),
                tile_def_name: "market".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("market"),
                min_workers: 0,
                max_workers: 1,
                stock_update_frequency_secs: 20.0,
                effect_radius: 5,
                resources_required: ResourceKinds::with_kinds(ResourceKind::foods()),
            },
            producer_rice_farm: ProducerConfig {
                name: "Rice Farm".to_string(),
                tile_def_name: "rice_farm".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("rice_farm"),
                min_workers: 0,
                max_workers: 1,
                production_output_frequency_secs: 20.0,
                production_output: ResourceKind::Rice,
                production_capacity: 5,
                resources_required: ResourceKinds::none(),
                resources_capacity: 0,
                storage_buildings_accepted: BuildingKind::Granary,
            },
            producer_livestock_farm: ProducerConfig {
                name: "Livestock Farm".to_string(),
                tile_def_name: "livestock_farm".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("livestock_farm"),
                min_workers: 0,
                max_workers: 1,
                production_output_frequency_secs: 20.0,
                production_output: ResourceKind::Meat,
                production_capacity: 5,
                resources_required: ResourceKinds::none(),
                resources_capacity: 0,
                storage_buildings_accepted: BuildingKind::Granary,
            },
            storage_yard: StorageConfig {
                name: "Storage Yard".to_string(),
                tile_def_name: "storage_yard".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("storage_yard"),
                min_workers: 0,
                max_workers: 1,
                resources_accepted: ResourceKinds::all(),
                num_slots: 8,
                slot_capacity: 4,
            },
            storage_granary: StorageConfig {
                name: "Granary".to_string(),
                tile_def_name: "granary".to_string(),
                tile_def_name_hash: hash::fnv1a_from_str("granary"),
                min_workers: 0,
                max_workers: 1,
                resources_accepted: ResourceKinds::with_kinds(ResourceKind::foods()),
                num_slots: 8,
                slot_capacity: 4,
            }
        }
    }

    pub fn find_house_config(&self) -> &HouseConfig {
        &self.house_cfg
    }

    pub fn find_house_level_config(&self, level: HouseLevel) -> &HouseLevelConfig {
        match level {
            HouseLevel::Level0 => &self.house0,
            HouseLevel::Level1 => &self.house1,
            HouseLevel::Level2 => &self.house2,
        }
    }

    pub fn find_producer_config(&self, kind: BuildingKind, tile_name: &str, tile_name_hash: StringHash) -> &ProducerConfig {
        if kind == BuildingKind::Farm {
            if tile_name_hash == hash::fnv1a_from_str("rice_farm") {
                &self.producer_rice_farm
            } else if tile_name_hash == hash::fnv1a_from_str("livestock_farm") {
                &self.producer_livestock_farm
            } else { panic!("Unknown farm tile: '{}'", tile_name) }
        } else { panic!("No producer!") }
    }

    pub fn find_service_config(&self, kind: BuildingKind) -> &ServiceConfig {
        if kind == BuildingKind::WellSmall {
            &self.service_well_small
        } else if kind == BuildingKind::WellBig {
            &self.service_well_big
        } else if kind == BuildingKind::Market {
            &self.service_market
        } else { panic!("No service!") }
    }

    pub fn find_storage_config(&self, kind: BuildingKind) -> &StorageConfig {
        if kind == BuildingKind::Granary {
            &self.storage_granary
        } else if kind == BuildingKind::StorageYard {
            &self.storage_yard
        } else { panic!("No storage!") }
    }
}

// ----------------------------------------------
// Helper functions
// ----------------------------------------------

pub fn instantiate<'config>(tile: &Tile, configs: &'config BuildingConfigs) -> Option<Building<'config>> {
    // TODO: Temporary
    let tile_name_hash = tile.tile_def().hash;
    if tile.name() == "well_small" {
        let config = configs.find_service_config(BuildingKind::WellSmall);
        Some(Building::new(
            BuildingKind::WellSmall,
            tile.cell_range(),
            BuildingArchetype::new_service(ServiceBuilding::new(config))
        ))
    } else if tile.name() == "well_big" {
        let config = configs.find_service_config(BuildingKind::WellBig);
        Some(Building::new(
            BuildingKind::WellBig,
            tile.cell_range(),
            BuildingArchetype::new_service(ServiceBuilding::new(config))
        ))
    } else if tile.name() == "market" {
        let config = configs.find_service_config(BuildingKind::Market);
        Some(Building::new(
            BuildingKind::Market,
            tile.cell_range(),
            BuildingArchetype::new_service(ServiceBuilding::new(config))
        ))
    } else if tile.name() == "house0" {
        let config = configs.find_house_config();
        Some(Building::new(
            BuildingKind::House,
            tile.cell_range(),
            BuildingArchetype::new_house(HouseBuilding::new(HouseLevel::Level0, config, configs))
        ))
    } else if tile.name() == "rice_farm" || tile.name() == "livestock_farm" {
        let config = configs.find_producer_config(BuildingKind::Farm, tile.name(), tile_name_hash);
        Some(Building::new(
            BuildingKind::Farm,
            tile.cell_range(),
            BuildingArchetype::new_producer(ProducerBuilding::new(config))
        ))
    } else if tile.name() == "granary" {
        let config = configs.find_storage_config(BuildingKind::Granary);
        Some(Building::new(
            BuildingKind::Granary,
            tile.cell_range(),
            BuildingArchetype::new_storage(StorageBuilding::new(config))
        ))
    } else if tile.name() == "storage_yard" {
        let config = configs.find_storage_config(BuildingKind::StorageYard);
        Some(Building::new(
            BuildingKind::StorageYard,
            tile.cell_range(),
            BuildingArchetype::new_storage(StorageBuilding::new(config))
        ))
    } else {
        eprintln!("Unknown building tile!");
        None
    }
}
