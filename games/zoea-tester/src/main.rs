use zoea_engine::atlas::Atlas;
use zoea_engine::game::GameEngine;
use zoea_engine::rendering::uv_rectangle::UvRectangle;
use zoea_engine::scene::Scene;

fn main() {
    let mut engine = GameEngine::default();

    let mut atlas = Atlas::new(
        "./assets/terrain_tiles_v2.png",
        UvRectangle::from_size(320.0, 512.0),
    );
    let id_1 = atlas.add_sprite(0, 0, 32, 32);
    let id_2 = atlas.add_sprite(0, 32, 32, 32);
    let id_3 = atlas.add_sprite(32, 32, 32, 32);

    let mut scene = Scene::default();

    //let mut entities = vec![];

    let uv_1 = atlas.get_sprite(id_1).unwrap().clone();

    engine.add_atlas(atlas);

    // entities.push(TempEntity {
    //     sprite: Sprite {
    //         id: AssetId(0),
    //         uv_rectangle: uv_1,
    //     },
    //     transform: Transform::from(Position::new(100.0, 100.0)),
    // });
    //
    // entities.push(TempEntity {
    //     sprite: Sprite {
    //         id: AssetId(0),
    //         uv_rectangle: uv_1,
    //     },
    //     transform: Transform::new(Position::new(100.0, 200.0), Rotation::Degrees(180.0), Scale::from(100.0)),
    // });
    //
    // scene.add_entities(entities);

    let scene_id = engine.add_scene(scene);

    engine.select_scene(scene_id);

    engine.start()
}
