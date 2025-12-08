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

const TICKS_FOR_DELETION: u32 = 60;

use crate::Resolution;
use crate::engine::config::Config;
use crate::engine::scene::Scene;
use crate::engine::scene::game_object::{Object, GameObject, ObjectKind};
use crate::interface::{ObjectWithImage, main_char_height, main_char_width, tile_height, tile_width};
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
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

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
                SceneManager::new(scene, end_scene),
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
                    let empty_object = create_obj_with_img(EMPTY, 0, 0, false, ObjectKind::Enemy);
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
                    - (keys_pressed_clone.s.load(Ordering::Relaxed) as i32) * 3;

                // let coef = 3;
                // dx_keys *= coef;
                // dy_keys *= coef;

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

                let objs1 = renderer.write().unwrap().scene_manager.active_scene.get_game_objects();
                let objs2 = objs1.clone();

                let main_pos = match renderer.write()
                    .unwrap()
                    .scene_manager
                    .active_scene
                    .main_object
                    .get_position()
                    .map(|pos| pos.clone()) {  // Clone if needed
                        Ok(pos) => pos,
                        _ => continue,
                    };

                let main_poss = Position {
                    x: main_pos.x + (WIDTH as i32) / 2 - (main_char_width as i32)/ 2,
                    y: main_pos.y + -(HEIGHT as i32) / 2 + (main_char_height as i32) / 2,
                    z: main_pos.z,
                    is_relative: false,
                };


                let mut have_touched = false;
                for (id, obj) in &objs1 {
                    let other_pos = obj.position;
                    if check_intersect(main_poss.x, main_poss.y, main_char_width, main_char_height,
                                        other_pos.x, other_pos.y, tile_width, tile_height) {
                        // println!("main obj and {}", id);
                        pending_deletions.push((*id, 0));
                        have_touched = true;
                    }
                }

                if (!have_touched) {
                    let vector_move = (0, -300);

                    //
                    renderer
                        .write()
                        .unwrap()
                        .scene_manager
                        .active_scene
                        .main_object
                        .add_position((vector_move.0, vector_move.1));

                }

                // Update counters and delete when ready
                pending_deletions.retain_mut(|(id, counter)| {
                    *counter += 1;
                    if *counter >= TICKS_FOR_DELETION {
                        if let Ok(mut guard) = renderer.write() {
                            guard.scene_manager.active_scene.delete_game_object_by_uid(*id);
                        }
                        false // Remove from list
                    } else {
                        true // Keep in list
                    }
                });


                let mut count = 0;
                for (id1, obj1) in &objs1 {

                    // treat other objects
                    for (id2, obj2) in &objs2 {
                        if id1 == id2 { continue };
                        let pos1 = obj1.position;
                        let pos2 = obj2.position;

                        if obj1.kind == ObjectKind::Player &&
                            check_intersect(pos1.x, pos1.y, main_char_width, main_char_height,
                                        pos2.x, pos2.y, tile_width, tile_height) {
                            println!("{} and {}", id1, id2);
                        }
                    }
                }
                // println!("found {} objects", count);


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
