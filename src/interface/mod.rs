//! Helper utilities for creating game objects, initializing scenes, and setting up the game engine.
//!
//! This module provides convenience functions to build vectors of game objects from simplified
//! image-based object definitions (`ObjectWithImage`), initialize scenes with those objects,
//! and create configured instances of the game engine.
//!
//! These functions support workflow from asset loading to scene setup to engine initialization.
//! 

/// Screen dimensions constants. COPIED
pub const WIDTH: u32 = 400;
pub const HEIGHT: u32 = 400;

pub const tile_width: i32 = 93;
pub const tile_height: i32 = 57;
pub const size: i32 = 6;
pub const main_char_width: i32 = 76;
pub const main_char_height: i32 = 46;

use crate::engine::scene::game_object::{ObjectKind, components::{Component, ComponentType}};

pub const EMPTY: &'static str = "src/bin/resources/empty.png";

use image::ImageReader;

use crate::{
    Resolution,
    engine::{
        Engine, GameEngine,
        config::{Config, EngineConfig},
        scene::{
            Scene,
            game_object::{GameObject, Object, Position, components::sprite::Sprite},
        },
        scene_manager::EndScene,
    },
};

/// Represents a simplified interface to a game object with associated image and placement.
///
/// Encapsulates the image filepath and positional attributes along with shadow flag
/// used for sprite rendering.
#[derive(Clone)]
pub struct ObjectWithImage<'a> {
    /// The filepath to the sprite's image.
    image_path: &'a str,
    /// The x-coordinate position in the game world.
    x: i32,
    /// The y-coordinate position in the game world.
    y: i32,
    /// Whether the sprite should cast a shadow.
    has_shadow: bool,
    /// Object kind
    kind: ObjectKind,
}

/// Converts a slice of `ObjectWithImage` entries into a vector of fully constructed `GameObject`s.
///
/// Assigns z-coordinates incrementally to arrange objects from farthest (z=1) to closest.
/// Loads sprite images from their file paths and creates corresponding sprite components.
///
/// # Parameters
/// - `objs`: Slice of simplified object image descriptions.
///
/// # Returns
/// Vector of fully constructed game objects ready for scene insertion.
pub fn create_gameobj_vec(objs: &[ObjectWithImage]) -> Vec<GameObject> {
    let mut res = Vec::new();
    let mut z_coord = 1;
    for obj in objs {
        let gameobj = GameObject::new(
            vec![Box::new(Sprite::new(
                Some(ImageReader::open(obj.image_path).unwrap().decode().unwrap()),
                obj.has_shadow,
                (0, 0),
            ))],
            None,
            Position {
                x: obj.x,
                y: obj.y,
                z: z_coord,
                is_relative: false,
            },
            obj.kind.clone(),
        );
        res.push(gameobj);
        z_coord += 1;
    }
    res
}

/// Creates an `ObjectWithImage` instance from image path and position data.
///
/// Encapsulates the essential information needed to create a game object sprite.
///
/// # Parameters
/// - `image_path`: File path to the sprite image.
/// - `x`, `y`: Position coordinates.
/// - `has_shadow`: Whether the sprite casts shadow.
///
/// # Returns
/// A new `ObjectWithImage` instance.
pub fn create_obj_with_img(image_path: &str, x: i32, y: i32, has_shadow: bool, kind: ObjectKind) -> ObjectWithImage {
    ObjectWithImage {
        image_path,
        x,
        y,
        has_shadow,
        kind
    }
}

/// Initializes a `Scene` from a slice of background objects and a single main object.
///
/// Converts all background objects into game objects with sprites and setups the main object
/// with its sprite and offset.
///
/// # Parameters
/// - `objs`: Slice of background `ObjectWithImage`.
/// - `main_obj`: The main object displayed in the scene.
///
/// # Returns
/// A full `Scene` instance initialized and ready for rendering.
pub fn init_scene(objs: &[ObjectWithImage], main_obj: ObjectWithImage) -> (Scene, Vec<GameObject>) {
    let game_objs = create_gameobj_vec(objs);
    let game_objs_clone = game_objs.clone();
    (Scene::new(
        game_objs,
        vec![Box::new(Sprite::new(
            Some(
                ImageReader::open(main_obj.image_path)
                    .unwrap()
                    .decode()
                    .unwrap(),
            ),
            true,
            // CHANGE this centers white_ship at the center of the screen, because x and y start from topleft corner
            (((WIDTH as i32) / 2 - (main_char_width as i32)/ 2), (-(HEIGHT as i32) / 2 + (main_char_height as i32) / 2)),
            // (360/2 - 76/2, -360/2 + 46/2),
            // (0, 0),
        ))],
        Position {
            x: main_obj.x,
            y: main_obj.y,
            z: 0,
            is_relative: false,
        },
    ), game_objs_clone)
}

/// Creates and initializes a `GameEngine` instance using the given scene and resolution.
///
/// Wraps configuration creation and engine construction in one step.
///
/// # Parameters
/// - `scene`: The initial scene the engine will manage.
/// - `width`, `height`: Screen resolution dimensions.
///
/// # Returns
/// A fully initialized `GameEngine` ready to run.
pub fn init_engine(scene: Scene, end_scene: EndScene, width: u32, height: u32) -> GameEngine {
    GameEngine::new(
        Box::new(EngineConfig::new(Resolution::new(width, height))),
        scene,
        end_scene,
    )
}

pub fn init_end_scene(image_path: &str, timeout_ms: Option<u64>) -> EndScene {
    let background = Some(ImageReader::open(image_path).unwrap().decode().unwrap());
    EndScene::new(background, timeout_ms)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_create_obj_with_img() {
        let obj = create_obj_with_img("./image", 100, 100, true, ObjectKind::Enemy);
        assert_eq!(obj.image_path, "./image");
        assert_eq!(obj.x, 100);
        assert_eq!(obj.y, 100);
        assert!(obj.has_shadow);
    }

    #[test]
    fn test_create_gameobj_vec() {
        let objs = [create_obj_with_img(
            "./resources/perf_diag.png",
            200,
            200,
            false,
            ObjectKind::Enemy
        )];
        let owi = create_gameobj_vec(&objs);
        assert_eq!(owi.len(), objs.len());
        assert_eq!(objs[0].x, owi[0].position.x);
        assert_eq!(objs[0].y, owi[0].position.y);
    }

    #[test]
    fn test_init_scene() {
        let objs = [create_obj_with_img(
            "./resources/perf_diag.png",
            200,
            200,
            false,
            ObjectKind::Enemy
        )];
        let main_obj = create_obj_with_img("./resources/perf_diag.png", 300, 300, true, ObjectKind::Player);
        let main_obj_x = main_obj.x;
        let main_obj_y = main_obj.y;
        let scene = init_scene(&objs, main_obj);
        assert_eq!(scene.0.main_object.position.x, main_obj_x);
        assert_eq!(scene.0.main_object.position.y, main_obj_y);
    }
}
