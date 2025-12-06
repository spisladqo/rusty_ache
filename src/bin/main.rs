use rusty_ache::engine::Engine;
use rusty_ache::engine::scene::game_object::GameObject;
use rusty_ache::engine::scene::game_object::components::script::Script;
use rusty_ache::engine::scene::game_object::position::Position;
use rusty_ache::interface::{ObjectWithImage, create_obj_with_img, init_end_scene, init_engine, init_scene};
use rusty_ache::screen::{HEIGHT, WIDTH};

const tile_width: i32 = 93;
const tile_height: i32 = 57;
const size: i32 = 2;

fn create_tile_objs(layer_num: i32, layer_gap_px: i32) -> Vec<ObjectWithImage<'static>> {
    let mut tile_objs = Vec::new();

    // Calculate center of screen
    let center_x: i32 = (WIDTH as i32) / 2 - (tile_width as i32)/ 2;
    let center_y: i32 = -(HEIGHT as i32) / 2 + (tile_height as i32) / 2;

    for layer in 0..layer_num {
        // Calculate vertical offset for this layer
        let layer_y_offset = layer * layer_gap_px;

        for sum in 0..=size*2 {
            let tiles_in_row = if sum <= size {
                sum + 1
            } else {
                size * 2 - sum + 1
            };

            let start_q = if sum <= size { 0 } else { sum - size };

            for i in 0..tiles_in_row {
                let q = start_q + i;
                let r = sum - q;

                // Calculate hexagonal grid coordinates relative to center
                let x_offset = (q as i32 * (tile_width / 2)) - (r as i32 * (tile_width / 2));
                let mut y_offset = (q as i32 * (tile_height / 2)) + (r as i32 * (tile_height / 2));
                
                // Apply layer offset to Y coordinate
                y_offset += layer_y_offset;

                // Calculate final position centered on screen
                let x = center_x + x_offset;
                let y = center_y + y_offset;

                println!("Layer {}: position of tile object is ({}, {})", layer, x, y);

                let tile_obj = create_obj_with_img("src/bin/resources/tile3.png", x, y, false);
                tile_objs.push(tile_obj);
            }
        }
    }
    
    return tile_objs;
}

fn main() {
    // let tower_obj = create_obj_with_img("src/bin/resources/tower.png", 82, 37, true);
    // let junk_house_obj = create_obj_with_img("src/bin/resources/junk_house.png", 150, -150, true);
    // let pool_house_obj = create_obj_with_img("src/bin/resources/pool_house.png", 15, -25, true);
    // let tall_house_obj = create_obj_with_img("src/bin/resources/tall_house.png", 210, -80, true);
    // let skyscraper_obj = create_obj_with_img("src/bin/resources/skyscraper.png", 150, 55, true);
    // let cabin_obj = create_obj_with_img("src/bin/resources/cabin.png", 280, -60, true);
    // CHANGE
    let main_ship_obj = create_obj_with_img("src/bin/resources/white_ship.png", 0, 50, true);

    let tiles_vec = create_tile_objs(1, 600);
    let tiles_slice : &[ObjectWithImage] = &tiles_vec;

    // let hermit_house_obj = create_obj_with_img("src/bin/resources/junk_house.png", 400, 240, true);

    let (mut scene, game_objs) = init_scene(
        tiles_slice,
        main_ship_obj,
    );

    for obj in game_objs {
        let id = (if let Some(uid) = scene.get_game_object_uid(obj) {uid} else { continue; }) ;
        println!("{}", id);
        scene.delete_game_object_by_uid(id);
    }

    let end_scene = init_end_scene("src/bin/resources/game_over.jpg", None);
    let mut engine = init_engine(scene, end_scene, WIDTH, HEIGHT);

    let main_pos_arc = engine.main_pos.clone();
    let end_scene_flag = engine.is_end_scene_active.clone();
    std::thread::spawn(move || {
        loop {
            let (x, y) = *main_pos_arc.read().unwrap();
            // println!("position of main object is ({}, {})", x, y);
            if x > 150 {
                end_scene_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
    });

    engine.render().unwrap();
    engine.run().unwrap()
}

#[derive(Clone)]
pub struct MyScript {
    is_downed: bool,
}

impl Script for MyScript {
    fn new(is_downed: bool) -> MyScript {
        MyScript { is_downed }
    }

    fn action(&mut self, game_object: &mut GameObject) {
        if !self.is_downed {
            game_object.position = Position {
                x: game_object.position.x,
                y: game_object.position.y - 1,
                z: game_object.position.z,
                is_relative: game_object.position.is_relative,
            };
            self.is_downed = true;
        } else {
            game_object.position = Position {
                x: game_object.position.x,
                y: game_object.position.y + 1,
                z: game_object.position.z,
                is_relative: game_object.position.is_relative,
            };
            self.is_downed = false;
        }
    }

    fn clone_box(&self) -> Box<dyn Script + Send + Sync> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use rusty_ache::engine::scene::game_object::{GameObject, Object, position::Position};

    use crate::{MyScript, Script};

    #[test]
    fn test_new_script() {
        let script = MyScript::new(false);
        assert!(!script.is_downed)
    }

    #[test]
    fn test_actions_is_downed_false() {
        let mut script = MyScript::new(false);
        let position = Position {
            x: 15,
            y: 25,
            z: 35,
            is_relative: false,
        };
        let game_object = &mut GameObject::new(vec![], None, position);
        script.action(game_object);
        assert_eq!(game_object.position.x, 15);
        assert_eq!(game_object.position.y, 24);
        assert_eq!(game_object.position.z, 35);
    }

    #[test]
    fn test_actions_is_downed_true() {
        let mut script = MyScript::new(true);
        let position = Position {
            x: 15,
            y: 25,
            z: 35,
            is_relative: false,
        };
        let game_object = &mut GameObject::new(vec![], None, position);
        script.action(game_object);
        assert_eq!(game_object.position.x, 15);
        assert_eq!(game_object.position.y, 26);
        assert_eq!(game_object.position.z, 35);
    }
}
