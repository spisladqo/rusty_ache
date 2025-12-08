//! Main engine module defining engine traits and implementation.
//!
//! Provides abstractions for engine lifecycle and rendering, as well as
//! a concrete `GameEngine` implementation which manages rendering, scene management,
//! input handling, and the event loop.
//!
//! This module integrates configurations, scenes, rendering, and input processing
//! to provide the core game engine loop and functionality.

pub mod config;
pub mod input;
pub mod scene;
pub mod scene_manager;
pub mod scripts;

const TICKS_FOR_DELETION: u32 = 20;

use crate::Resolution;
use crate::engine::config::Config;
use crate::engine::scene::Scene;
use crate::engine::scene::game_object::{Object, GameObject, ObjectKind};
use crate::interface::{ObjectWithImage, death_y, init_end_scene, init_engine, layer_gap, main_char_height, main_char_width, tile_height, tile_width};
use crate::engine::scene_manager::{EndScene, SceneManager};
use crate::engine::scripts::main_obj_script;
use crate::engine::scene::game_object::position::Position;
use crate::interface::{create_obj_with_img, init_scene};
use crate::interface::create_gameobj_vec;
use crate::render::renderer::{DEFAULT_BACKGROUND_COLOR, Renderer};
use crate::screen::{App, HEIGHT, WIDTH};
// use crate::end_scene::EndScene;
//use image::ImageReader;
use std::io::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{thread, vec};
use pixels::wgpu::core::binding_model::CreateBindGroupError;
use pixels::wgpu::core::resource::CreateBufferError;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use std::collections::HashMap;

pub const EMPTY: &'static str = "src/bin/resources/empty.png";

/// Trait defining essential engine behavior.
///
/// Abstracts an engine capable of managing an active scene, performing rendering,
/// running its main loop, and supporting dynamic configuration.
pub trait Engine {
    /// Sets the currently active scene within the engine.
    fn set_active_scene(&mut self, new_scene: Scene, end_scene: EndScene) -> Result<(), Error>;

    /// Performs a rendering pass.
    fn render(&mut self) -> Result<(), Error>;

    /// Starts and runs the engine main loop.
    fn run(&mut self) -> Result<(), Error>;

    /// Creates a new engine instance from configuration and initial scene.
    fn new(config: Box<dyn Config + Send>, scene: Scene, end_scene: EndScene) -> Self
    where
        Self: Sized;
}

/// Concrete implementation of the game engine.
///
/// Holds a thread-safe renderer reference, manages scenes and input handling,
/// runs the main event loop and coordinates rendering.
pub struct GameEngine {
    //config: Box<dyn Config + Send>,
    pub render: Arc<RwLock<Renderer>>,
    pub main_pos: Arc<RwLock<(i32, i32)>>,
    pub is_end_scene_active: Arc<AtomicBool>,
}

impl Engine for GameEngine {
    /// Sets the active scene inside the renderer's scene manager.
    fn set_active_scene(&mut self, new_scene: Scene, end_scene: EndScene) -> Result<(), Error> {
        self.render.write().unwrap().scene_manager = SceneManager::new(new_scene, end_scene);

        Ok(())
    }

    /// Delegates rendering to the internal Renderer instance.
    fn render(&mut self) -> Result<(), Error> {
        self.render.write().unwrap().render();
        Ok(())
    }

    /// Creates a new GameEngine using provided config and scene.
    ///
    /// Initializes the Renderer with the resolution and the scene manager.
    fn new(config: Box<dyn Config + 'static + Send>, scene: Scene, end_scene: EndScene) -> Self
    where
        Self: Sized,
    {
        let res = config.get_resolution();
        let main_obj_x = scene.main_object.position.x;
        let main_obj_y = scene.main_object.position.y;
        GameEngine {
            //config,
            render: Arc::new(RwLock::from(Renderer::new(
                res,
                /*Some(ImageReader::open("src/bin/resources/tile2.png")
                .unwrap()
                .decode()
                .unwrap())*/
                None,
                SceneManager::new(scene.clone(), end_scene)
            ))),
            main_pos: Arc::new(RwLock::from((main_obj_x, main_obj_y))),
            is_end_scene_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Runs the game engine event loop.
    ///
    /// Sets up shared state for pixel data, window, and input keys.
    /// Spawns a producer thread that updates the main object's position based on key input
    /// and triggers rendering updates.
    /// Runs the `winit` event loop with the associated GUI application.
    fn run(&mut self) -> Result<(), Error> {
        let initial_resolution = Resolution {
            width: WIDTH,
            height: HEIGHT,
        };
        let initial_pixels = vec![
            DEFAULT_BACKGROUND_COLOR;
            (initial_resolution.width * initial_resolution.height) as usize
        ];

        let shared_pixel_data = Arc::new(RwLock::new(initial_pixels));
        let shared_window = Arc::new(RwLock::new(None));

        let shared_pixel_data_clone = shared_pixel_data.clone();
        let shared_window_clone = shared_window.clone();

        let mut is_win = false;

        let mut tiles: Vec<(u32, GameObject)> = Vec::new();
        let mut enemies: Vec<(u32, GameObject)> = Vec::new();
        let mut is_initialized = false;
        let mut random_movement_counter: i32 = 0;
        let mut enemy_directions: HashMap<u32, (i32, i32)> = HashMap::new();

        let mut app = App::new(shared_pixel_data, shared_window);
        //let key_pressed_clone = app.key_pressed.clone();
        let keys_pressed_clone = app.keys_pressed.clone();
        let renderer = self.render.clone();
        let main_pos_arc = self.main_pos.clone();

        let is_end_scene_active = self.is_end_scene_active.clone();
        let start_scene = self
            .render
            .read()
            .unwrap()
            .scene_manager
            .active_scene
            .clone();
        let new_background = renderer
            .read()
            .unwrap()
            .scene_manager
            .end_scene
            .background
            .clone();

        let mut pending_deletions: Vec<(usize, u32)> = Vec::new();

        let main_pos_arc1 = self.main_pos.clone();
        let end_scene_flag = self.is_end_scene_active.clone();
        // std::thread::spawn(move || {
        //     loop {
        //         let (x, y) = *main_pos_arc1.read().unwrap();
        //         if y < -100 {
        //             end_scene_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        //             break;
        //         }

        //         std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 checks per second
        //     }
        // });

        thread::spawn(move || {
            let window_arc: Arc<Window> = loop {
                if let Some(arc) = shared_window_clone.read().unwrap().clone() {
                    break arc;
                }
                thread::sleep(Duration::from_millis(50));
            };

            thread::sleep(Duration::from_secs(3));

            //dbg!("Producer has started");

            let screen_size = (WIDTH * HEIGHT) as usize;
            loop {
                if is_end_scene_active.load(Ordering::SeqCst) {
                    let prev_background = renderer
                        .write()
                        .unwrap()
                        .set_background(new_background.clone());
                    let empty_object = create_obj_with_img(EMPTY, 0, 0, false, ObjectKind::Stub);
                    let (scene, mut game_objs) = init_scene(&[], empty_object);
                    let timeout_ms = renderer.read().unwrap().scene_manager.end_scene.timeout_ms;
                    renderer.write().unwrap().scene_manager =
                        SceneManager::new(scene, EndScene::new(new_background.clone(), timeout_ms));
                    let pause_until: Instant = if timeout_ms.is_some() {
                        Instant::now() + Duration::from_millis(timeout_ms.unwrap())
                    } else {
                        Instant::now()
                    };

                    loop {
                        renderer.write().unwrap().render();
                        match renderer.write().unwrap().emit() {
                            Some(colors) => {
                                let mut pixels = shared_pixel_data_clone
                                    .write()
                                    .expect("Producer couldn't lock pixel data");

                                for (idx, p) in pixels.iter_mut().take(screen_size).enumerate() {
                                    *p = colors[idx];
                                }

                                window_arc.request_redraw();
                            }
                            None => {
                                continue;
                            }
                        }
                        if timeout_ms.is_some() && Instant::now() >= pause_until {
                            break;
                        }
                    }
                    renderer.write().unwrap().scene_manager = SceneManager::new(
                        start_scene.clone(),
                        renderer.read().unwrap().scene_manager.end_scene.clone(),
                    );
                    renderer
                        .write()
                        .unwrap()
                        .set_background(prev_background)
                        .unwrap();
                    is_end_scene_active.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                /*let vector_move = match *key_pressed_clone.read().unwrap() {
                    Some(KeyCode::KeyW) => (0, 1),
                    Some(KeyCode::KeyA) => (-1, 0),
                    Some(KeyCode::KeyS) => (0, -1),
                    Some(KeyCode::KeyD) => (1, 0),
                    _ => (0, 0),
                };*/
                let mut dx_keys = (keys_pressed_clone.d.load(Ordering::Relaxed) as i32)
                    - (keys_pressed_clone.a.load(Ordering::Relaxed) as i32);
                let mut dy_keys = (keys_pressed_clone.w.load(Ordering::Relaxed) as i32)
                    - (keys_pressed_clone.s.load(Ordering::Relaxed) as i32);

                let coef = 5;
                dx_keys *= coef;
                dy_keys *= coef;

                let (dx_script, dy_script) = main_obj_script();
                let vector_move = (dx_keys + dx_script, dy_keys + dy_script);

                //
                renderer
                    .write()
                    .unwrap()
                    .scene_manager
                    .active_scene
                    .main_object
                    .add_position((vector_move.0, vector_move.1));

                let (main_pos, objs1) = {
                    let guard = renderer.read().unwrap();
                    let scene = &guard.scene_manager.active_scene;

                    let main_pos = match scene.main_object.get_position() {
                        Ok(pos) => pos.clone(),
                        Err(_) => {
                            break;
                        }
                    };

                    let objs = scene.get_game_objects().clone();
                    
                    (main_pos, objs)
                };

                let main_poss = Position {
                    x: main_pos.x + (WIDTH as i32) / 2 - (main_char_width as i32) / 2,
                    y: main_pos.y + -(HEIGHT as i32) / 2 + (main_char_height as i32) / 2,
                    z: main_pos.z,
                    is_relative: false,
                };

                let mut have_touched_tile = false;

                let tile_positions: Vec<(usize, Position)> = objs1.iter()
                    .filter(|(_, obj)| obj.get_kind() == &ObjectKind::Tile)
                    .map(|(id, obj)| (*id, obj.position))
                    .collect();

                for (tile_id, tile_pos) in &tile_positions {
                    if check_intersect(main_poss.x + 20, main_poss.y + 20, 40, 20,
                                    tile_pos.x, tile_pos.y, tile_width, tile_height) {
                        pending_deletions.push((*tile_id, 0));
                        have_touched_tile = true;
                    }
                }

                if !have_touched_tile {
                    let vector_move = (0, -layer_gap);
                    renderer.write()
                            .unwrap()
                            .scene_manager.active_scene
                            .main_object
                            .add_position(vector_move);
                }

                fn has_tile_in_direction(current_pos: Position, direction: (i32, i32), 
                                        tile_positions: &[(usize, Position)], 
                                        enemy_width: i32, enemy_height: i32) -> bool {
                    let check_pos = Position {
                        x: current_pos.x + direction.0,
                        y: current_pos.y + direction.1,
                        ..current_pos
                    };
                    
                    tile_positions.iter().any(|(_, tile_pos)| {
                        check_intersect(
                            check_pos.x + 20, check_pos.y + 20, 40, 20,
                            tile_pos.x, tile_pos.y, tile_width, tile_height
                        )
                    })
                }

                let enemies = objs1.iter()
                    .filter(|(_, obj)| obj.get_kind() == &ObjectKind::Enemy);

                let num_enemies = enemies.count();
                if num_enemies == 0 {
                    let win_scene = init_end_scene("src/bin/resources/dragons_death.jpg", None);
                    renderer.write().unwrap().scene_manager.end_scene = win_scene;

                    is_end_scene_active.store(true, std::sync::atomic::Ordering::SeqCst);
                    println!("You're the last one left, you win!\n");
                    continue;
                }


                if random_movement_counter == 0 {
                    random_movement_counter = 10;
                    enemy_directions.clear();

                    for (enemy_id, enemy_obj) in objs1.iter()
                        .filter(|(_, obj)| obj.get_kind() == &ObjectKind::Enemy) {
                        
                        let current_pos = enemy_obj.position;

                        let mut possible_directions = vec![
                            (1, 0),
                            (-1, 0),
                            (0, 1),
                            (0, -1),
                            (1, 1),
                            (-1, 1),
                            (1, -1),
                            (-1, -1),
                        ];

                        for dir in &mut possible_directions {
                            dir.0 *= coef;
                            dir.1 *= coef;
                        }

                        let safe_directions: Vec<(i32, i32)> = possible_directions.iter()
                            .filter(|&&dir| has_tile_in_direction(current_pos, dir, &tile_positions, main_char_width, main_char_height))
                            .cloned()
                            .collect();

                        let chosen_direction = if !safe_directions.is_empty() {
                            use rand::random;
                            let idx = (random::<u32>() as usize) % safe_directions.len();
                            safe_directions[idx]
                        } else {
                            (0, 0)
                        };

                        enemy_directions.insert(*enemy_id as u32, chosen_direction);
                    }
                }

                if random_movement_counter > 0 {
                    random_movement_counter -= 1;

                    if let Ok(mut guard) = renderer.write() {
                        for (enemy_id, direction) in &enemy_directions {
                            if let Some(enemy) = guard.scene_manager.active_scene.manager.game_objects
                                .iter_mut()
                                .find(|obj| *obj.0 == (*enemy_id as usize)) {
                                
                                enemy.1.add_position(*direction);
                            }
                        }
                    }
                }

                // Основная обработка врагов
                for (enemy_id, enemy_obj) in objs1.iter()
                    .filter(|(_, obj)| obj.get_kind() == &ObjectKind::Enemy) {
                    if enemy_obj.position.y < death_y {
                        if let Ok(mut guard) = renderer.write() {
                            guard.scene_manager.active_scene.delete_game_object_by_uid(*enemy_id);
                        }
                        continue;
                    }
                    
                    let mut enemy_touching_tile = false;
                    
                    for (tile_id, tile_pos) in &tile_positions {
                        if check_intersect(
                            enemy_obj.position.x, enemy_obj.position.y, main_char_width, main_char_height,
                            tile_pos.x, tile_pos.y, tile_width, tile_height
                        ) {
                            enemy_touching_tile = true;
                            
                            if !pending_deletions.iter().any(|(id, _)| *id == *tile_id) {
                                pending_deletions.push((*tile_id, 0));
                            }
                        }
                    }
                    
                    if !enemy_touching_tile {
                        if let Ok(mut guard) = renderer.write() {
                            if let Some(enemy) = guard.scene_manager.active_scene.manager.game_objects
                                .iter_mut()
                                .find(|obj| *obj.0 == *enemy_id) {
                                
                                let fall_vector = (0, -layer_gap);
                                enemy.1.add_position(fall_vector);
                            }
                        }
                    }
                }

                pending_deletions.retain_mut(|(id, counter)| {
                    *counter += 1;
                    if *counter >= TICKS_FOR_DELETION {
                        if let Ok(mut guard) = renderer.write() {
                            guard.scene_manager.active_scene.delete_game_object_by_uid(*id);
                        }
                        false
                    } else {
                        true
                    }
                });

                {
                    let pos = renderer
                        .read()
                        .unwrap()
                        .scene_manager
                        .active_scene
                        .main_object
                        .position;

                    *main_pos_arc.write().unwrap() = (pos.x, pos.y);
                }
                //


                renderer.write().unwrap().render();

                match renderer.write().unwrap().emit() {
                    Some(colors) => {
                        let mut pixels = shared_pixel_data_clone
                            .write()
                            .expect("Producer couldn't lock pixel data");

                        for (idx, p) in pixels.iter_mut().take(screen_size).enumerate() {
                            *p = colors[idx];
                        }

                        window_arc.request_redraw();
                    }
                    None => {
                        continue;
                    }
                }
            }
        });

        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);
        let _ = event_loop.run_app(&mut app);


        if is_win {
            println!("You're the last one left, you won!");
        } else {
            println!("You fell, that's a lose!");
        }

        Ok(())
    }
}

fn check_intersect(x1: i32, y1: i32, w1: i32, h1: i32, x2: i32, y2: i32, w2: i32, h2: i32) -> bool {
    if x1 > (x2 + w2) || x2 > (x1 + w1) {
        return false
    }

    if (y1 - h1) > y2 || (y2 - h2) > y1 {
        return false
    }
    
    return true
}

#[cfg(test)]
mod tests {
    use crate::{
        Resolution,
        engine::{config::EngineConfig, scene::game_object::Position},
    };

    use super::*;

    fn _create_config_with_resolution(
        width: u32,
        height: u32,
    ) -> Box<dyn config::Config + Send + 'static> {
        Box::new(EngineConfig::new(Resolution::new(width, height)))
    }

    fn _create_empty_scene() -> Scene {
        Scene::new(
            vec![],
            vec![],
            Position {
                x: 0,
                y: 0,
                z: 0,
                is_relative: false,
            },
        )
    }
}
