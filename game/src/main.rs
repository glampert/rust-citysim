#![allow(dead_code)]

mod app;
mod debug;
mod game;
mod imgui_ui;
mod render;
mod tile;
mod utils;

use imgui_ui::*;
use render::*;
use utils::{
    *,
    coords::*
};
use app::{
    *,
    input::*
};
use debug::{
    inspector::*,
    palette::*,
    settings::*
};
use tile::{
    camera::{self, *},
    rendering::{self, *},
    selection::*,
    placement::*,
    sets::*,
    map::*
};
use game::{
    sim::*,
    sim::world::*,
    building::{self, config::BuildingConfigs},
};

// ----------------------------------------------
// main()
// ----------------------------------------------

fn main() {
    let cwd = std::env::current_dir().unwrap();
    println!("The current directory is \"{}\".", cwd.display());

    let mut app = ApplicationBuilder::new()
        .window_title("CitySim")
        .window_size(Size::new(1024, 768))
        .fullscreen(false)
        .confine_cursor_to_window(camera::CONFINE_CURSOR_TO_WINDOW)
        .build();

    let input_sys = app.create_input_system();

    let mut render_sys = RenderSystemBuilder::new()
        .viewport_size(app.window_size())
        .clear_color(rendering::MAP_BACKGROUND_COLOR)
        .build();

    let mut ui_sys = UiSystem::new(&app);

    let tile_sets = TileSets::load(render_sys.texture_cache_mut());

    let mut tile_map = create_test_tile_map(&tile_sets);
    //let mut tile_map = TileMap::new(Size::new(64, 64), None);

    let building_configs = BuildingConfigs::load();
    let mut sim = Simulation::new();
    let mut world = World::new();

    // TODO: This is temporary while testing only. Map should start empty.
    tile_map.for_each_tile_mut(TileMapLayerKind::Objects, TileKind::Building, |tile| {
        if let Some(building) = building::config::instantiate(tile, &building_configs) {
            world.add_building(tile, building);
        }
    });

    let mut tile_selection = TileSelection::new();
    let mut tile_map_renderer = TileMapRenderer::new(rendering::DEFAULT_GRID_COLOR, 1.0);

    let mut camera = Camera::new(
        render_sys.viewport().size(),
        tile_map.size_in_cells(),
        camera::MIN_ZOOM,
        camera::Offset::Center);

    let mut tile_inspector_menu = TileInspectorMenu::new();
    let mut tile_palette_menu = TilePaletteMenu::new(true, render_sys.texture_cache_mut());
    let mut debug_settings_menu = DebugSettingsMenu::new(false);

    let mut frame_clock = FrameClock::new();

    while !app.should_quit() {
        frame_clock.begin_frame();

        let cursor_screen_pos = input_sys.cursor_pos();

        for event in app.poll_events() {
            match event {
                ApplicationEvent::Quit => {
                    app.request_quit();
                }
                ApplicationEvent::WindowResize(window_size) => {
                    render_sys.set_viewport_size(window_size);
                    camera.set_viewport_size(window_size);
                }
                ApplicationEvent::KeyInput(key, action, modifiers) => {
                    if ui_sys.on_key_input(key, action, modifiers).is_handled() {
                        continue;
                    }

                    if key == InputKey::Escape {
                        tile_inspector_menu.close();
                        tile_palette_menu.clear_selection();
                        tile_map.clear_selection(&mut tile_selection);
                    }
                }
                ApplicationEvent::CharInput(c) => {
                    if ui_sys.on_char_input(c).is_handled() {
                        continue;
                    }
                }
                ApplicationEvent::Scroll(amount) => {
                    if ui_sys.on_scroll(amount).is_handled() {
                        continue;
                    }

                    if amount.y < 0.0 {
                        camera.request_zoom(camera::Zoom::In);
                    } else if amount.y > 0.0 {
                        camera.request_zoom(camera::Zoom::Out);
                    }
                }
                ApplicationEvent::MouseButton(button, action, modifiers) => {
                    if ui_sys.on_mouse_click(button, action, modifiers).is_handled() {
                        continue;
                    }

                    if tile_palette_menu.has_selection() {
                        if tile_palette_menu.on_mouse_click(button, action).not_handled() {
                            tile_palette_menu.clear_selection();
                            tile_map.clear_selection(&mut tile_selection);
                        }
                    } else {
                        if tile_selection.on_mouse_click(button, action, cursor_screen_pos).not_handled() {
                            tile_palette_menu.clear_selection();
                            tile_map.clear_selection(&mut tile_selection);
                        }

                        if let Some(selected_tile) = tile_map.topmost_selected_tile(&tile_selection) {
                            if tile_inspector_menu.on_mouse_click(button, action, selected_tile).is_handled() {
                                continue;
                            }
                        }
                    }
                }
            }
        }

        sim.update(&mut world, &mut tile_map, &tile_sets, frame_clock.delta_time());

        camera.update_zooming(frame_clock.delta_time());

        // If we're not hovering over an ImGui menu...
        if !ui_sys.is_handling_mouse_input() {
            // Map scrolling:
            camera.update_scrolling(cursor_screen_pos, frame_clock.delta_time());

            // Tile hovering and selection:
            let placement_op = {
                if let Some(tile_def) = tile_palette_menu.current_selection(&tile_sets) {
                    PlacementOp::Place(tile_def)
                } else if tile_palette_menu.is_clear_selected() {
                    PlacementOp::Clear
                } else {
                    PlacementOp::None
                }
            };

            tile_map.update_selection(
                &mut tile_selection,
                cursor_screen_pos,
                camera.transform(),
                placement_op);
        }

        if tile_palette_menu.can_place_tile() {
            let placement_candidate = tile_palette_menu.current_selection(&tile_sets);

            let did_place_or_clear = {
                // If we have a selection place it, otherwise we want to try clearing the tile under the cursor.
                if let Some(tile_def) = placement_candidate {
                    let place_result = tile_map.try_place_tile_at_cursor(
                        cursor_screen_pos,
                        camera.transform(),
                        tile_def);

                    if let Some(tile) = place_result {
                        if tile_def.is(TileKind::Building) {
                            if let Some(building) = building::config::instantiate(tile, &building_configs) {
                                world.add_building(tile, building);
                            }
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    if let Some(tile) = tile_map.topmost_tile_at_cursor(cursor_screen_pos, camera.transform()) {
                        if tile.is(TileKind::Building | TileKind::Blocker) {
                            world.remove_building(tile);
                        }
                    }

                    tile_map.try_clear_tile_at_cursor(
                        cursor_screen_pos,
                        camera.transform())
                }
            };

            let placing_an_object = placement_candidate.map_or(false, 
                |def| def.is(TileKind::Object));

            let clearing_a_tile = tile_palette_menu.is_clear_selected();

            if did_place_or_clear && (placing_an_object || clearing_a_tile) {
                // Place or remove building/unit and exit tile placement mode.
                tile_palette_menu.clear_selection();
                tile_map.clear_selection(&mut tile_selection);
            }
        }

        let visible_range = camera.visible_cells_range();

        tile_map.update_anims(visible_range, frame_clock.delta_time());

        ui_sys.begin_frame(&app, &input_sys, frame_clock.delta_time());
        render_sys.begin_frame();

        let tile_render_stats = tile_map_renderer.draw_map(
            &mut render_sys,
            &ui_sys,
            &tile_map,
            camera.transform(),
            visible_range,
            debug_settings_menu.selected_render_flags());

        tile_selection.draw(&mut render_sys);

        tile_palette_menu.draw(
            &mut render_sys,
            &ui_sys,
            &tile_sets,
            cursor_screen_pos,
            camera.transform(),
            tile_selection.has_valid_placement(),
            debug_settings_menu.show_selection_bounds());

        tile_inspector_menu.draw(&mut sim, &mut world, &mut tile_map, &tile_sets, &ui_sys, camera.transform());
        debug_settings_menu.draw(&mut camera, &mut world, &mut tile_map_renderer, &mut tile_map, &tile_sets, &ui_sys);

        sim.draw_building_debug_popups(
            &mut world,
            &mut tile_map,
            &tile_sets,
            &ui_sys,
            camera.transform(),
            visible_range,
            frame_clock.delta_time(),
            debug_settings_menu.show_popup_messages());

        if debug_settings_menu.show_cursor_pos() {
            debug::utils::draw_cursor_overlay(&ui_sys, camera.transform());
        }

        if debug_settings_menu.show_screen_origin() {
            debug::utils::draw_screen_origin_marker(&mut render_sys);
        }

        let render_sys_stats = render_sys.end_frame();

        if debug_settings_menu.show_render_stats() {
            debug::utils::draw_render_stats(&ui_sys, &render_sys_stats, &tile_render_stats);
        }

        ui_sys.end_frame();

        app.present();

        frame_clock.end_frame();
    }
}

fn create_test_tile_map(tile_sets: &TileSets) -> TileMap {
    println!("Creating test tile map...");

    const MAP_WIDTH:  i32 = 8;
    const MAP_HEIGHT: i32 = 8;

    const G: i32 = 0; // grass
    const D: i32 = 1; // dirt
    const H: i32 = 2; // house
    const W: i32 = 3; // well_small
    const B: i32 = 4; // well_big
    const M: i32 = 5; // market

    const TILE_NAMES: [&str; 6] = [ "grass", "dirt", "house0", "well_small", "well_big", "market" ];
    const TILE_CATEGORIES: [&str; 6] = [ "ground", "ground", "buildings", "buildings", "buildings", "buildings" ];

    let find_tile = |layer_kind: TileMapLayerKind, tile_id: i32| {
        let tile_name = TILE_NAMES[tile_id as usize];
        let category_name = TILE_CATEGORIES[tile_id as usize];
        tile_sets.find_tile_def_by_name(layer_kind, category_name, tile_name)
    };

    const TERRAIN_LAYER_MAP: [i32; (MAP_WIDTH * MAP_HEIGHT) as usize] = [
        D,D,D,D,D,D,D,D, // <-- start, tile zero is the leftmost (top-left)
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,D,D,D,D,D,D,D,
    ];

    const BUILDINGS_LAYER_MAP: [i32; (MAP_WIDTH * MAP_HEIGHT) as usize] = [
        D,D,D,D,D,D,D,D, // <-- start, tile zero is the leftmost (top-left)
        D,H,G,B,G,M,G,D,
        D,G,G,G,G,G,G,D,
        D,G,W,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,G,G,G,G,G,G,D,
        D,D,D,D,D,D,D,D,
    ];

    let mut tile_map = TileMap::new(Size::new(MAP_WIDTH, MAP_HEIGHT), None);

    // Terrain:
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let tile_id = TERRAIN_LAYER_MAP[(x + (y * MAP_WIDTH)) as usize];
            if let Some(tile_def) = find_tile(TileMapLayerKind::Terrain, tile_id) {
                let place_result = tile_map.try_place_tile_in_layer(Cell::new(x, y), TileMapLayerKind::Terrain, tile_def);
                debug_assert!(place_result.is_some());
            }
        }
    }

    // Buildings:
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let tile_id = BUILDINGS_LAYER_MAP[(x + (y * MAP_WIDTH)) as usize];
            if tile_id == G || tile_id == D {
                    // ground/empty
            } else {
                // building tile
                if let Some(tile_def) = find_tile(TileMapLayerKind::Objects, tile_id) {
                    let place_result = tile_map.try_place_tile_in_layer(Cell::new(x, y), TileMapLayerKind::Objects, tile_def);
                    debug_assert!(place_result.is_some());
                }
            }
        }
    }

    tile_map
}
