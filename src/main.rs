use bevy::core_pipeline::bloom::BloomSettings;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    window::PrimaryWindow,
};
use bevy_rapier2d::prelude::*;
use rand;

const CHARACTER_SPEED: f32 = 41.0;
const CHARACTER_JUMP_FORCE: f32 = 51.0;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Character {
    speed: f32,
    jump_force: f32,
}

#[derive(Component)]
struct Health {
    health: i32,
    max_health: i32,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct WeaponDisplay;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(
                    // This sets image filtering to nearest
                    // This is done to prevent textures with low resolution (e.g. pixel art) from being blurred
                    // by linear filtering.
                    ImagePlugin::default_nearest(),
                )
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resizable: true,
                        title: "Pylon Recon Alpha".to_string(),
                        mode: bevy::window::WindowMode::BorderlessFullscreen,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
        )
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(12.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_input)
        //.add_systems(Update, modify_sprite_test)
        .run();
}

fn get_image_clone(
    mut res: &mut ResMut<Assets<Image>>,
    asset_server: &Res<AssetServer>,
    path: &str,
) -> Handle<Image> {
    let path_clone = path.to_string();
    // this function gets the image and adds a clone of it to the assets, that way you can modify the clone. that means you could have 100 entities with the same image, but modify each one differently
    let src_image = res.get_mut(asset_server.load(path_clone).id()).unwrap();
    let mut image = Image::new(
        src_image.texture_descriptor.size,
        src_image.texture_descriptor.dimension,
        src_image.data.clone(),
        src_image.texture_descriptor.format,
    );
    // add the clone to the assets
    let handle = res.add(image);
    handle
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings {
            intensity: 0.2,
            ..Default::default()
        },
        MainCamera,
    ));
    const player_half_width: f32 = 11.0 / 2.0;
    const player_half_height: f32 = 6.0;
    let cone_shape_option = Collider::convex_hull(&[
        // we have one point at bottom left, one at bottom right, then two near top middle. makes a cone shape
        Vec2::new(-player_half_width, -player_half_height + 2.0),
        Vec2::new(0., -player_half_height),
        Vec2::new(player_half_width, -player_half_height + 2.0),
        Vec2::new(0.5, player_half_height),
        Vec2::new(-0.5, player_half_height),
    ]);
    // its option, lets unwrap it
    let cone_shape = cone_shape_option.unwrap();
    let player = commands
        .spawn((
            SpriteBundle {
                texture: asset_server.load("cone.png"),
                transform: Transform::from_xyz(100., 0., 0.),
                ..Default::default()
            },
            Player,
            Health {
                health: 10,
                max_health: 10,
            },
            Character {
                jump_force: CHARACTER_JUMP_FORCE,
                speed: CHARACTER_SPEED,
            },
            cone_shape.clone(),
            RigidBody::Dynamic,
            Velocity::default(),
            LockedAxes::ROTATION_LOCKED,
            ExternalForce::default(),
        ))
        .id();
    let weapon = commands
        .spawn((
            SpriteBundle {
                texture: asset_server.load("weapon_2.png"),
                transform: Transform::from_xyz(0., -0.5, 0.),
                ..Default::default()
            },
            WeaponDisplay,
        ))
        .id();
    // child
    commands.entity(player).push_children(&[weapon]);
    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("cone.png"),
            transform: Transform::from_xyz(130., 0., 0.),
            ..Default::default()
        },
        Health {
            health: 10,
            max_health: 10,
        },
        Character {
            jump_force: CHARACTER_JUMP_FORCE,
            speed: CHARACTER_SPEED,
        },
        cone_shape.clone(),
        RigidBody::Dynamic,
        Velocity::default(),
        LockedAxes::ROTATION_LOCKED,
    ));
    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(500.0, 50.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -100.0, 0.0)));

    // create a bunch of walls near middle of ground for parkour
    commands
        .spawn(Collider::cuboid(5.0, 20.0))
        .insert(TransformBundle::from(Transform::from_xyz(10.0, -25.0, 0.0)));
    commands
        .spawn(Collider::cuboid(5.0, 20.0))
        .insert(TransformBundle::from(Transform::from_xyz(40.0, -25.0, 0.0)));
    commands
        .spawn(Collider::cuboid(5.0, 20.0))
        .insert(TransformBundle::from(Transform::from_xyz(70.0, -25.0, 0.0)));
    commands
        .spawn(Collider::cuboid(5.0, 20.0))
        .insert(TransformBundle::from(Transform::from_xyz(
            100.0, -25.0, 0.0,
        )));

    // 1000 rigidbody boxes stacked on Y axis
    for i in 0..50 {
        commands.spawn((
            Collider::cuboid(2.0, 2.0),
            RigidBody::Dynamic,
            TransformBundle::from(Transform::from_xyz(140.0, 1. + i as f32 * 4.1, 0.0)),
        ));
    }

    // staircase made of 1 unit tall and 15 unit wide boxes
    for i in 0..50 {
        commands.spawn((
            Collider::cuboid(15.0 / 2., 1.0 / 2.),
            TransformBundle::from(Transform::from_xyz(
                150.0 + i as f32 * 15.0,
                -10. + i as f32 * 1.0,
                0.0,
            )),
        ));
    }
}

fn keyboard_input(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    rapier_context: Res<RapierContext>,
    mut gizmos: Gizmos,
    mut query: Query<(
        &Player,
        &Character,
        &Health,
        &mut Velocity,
        &mut Transform,
        &mut Sprite,
        &mut ExternalForce,
    )>,
    mut camera_query: Query<(
        &MainCamera,
        &mut OrthographicProjection,
        &mut Transform,
        &GlobalTransform,
        &Camera,
        Without<Player>,
    )>,
    mut weapon_query: Query<(
        &WeaponDisplay,
        &mut Transform,
        &mut Sprite,
        &mut GlobalTransform,
        Without<Player>,
        Without<MainCamera>,
    )>,
    buttons: Res<Input<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // get player
    let mut stuff = query.single_mut();
    // get camera
    let mut camera: (
        &MainCamera,
        Mut<'_, OrthographicProjection>,
        Mut<'_, Transform>,
        &GlobalTransform,
        &Camera,
        _,
    ) = camera_query.single_mut();

    const RAY_DISTANCE: f32 = 0.2;

    // point 1 is underneath at middle
    const BOTTOM_VERTICAL_DISTANCE: f32 = 6.0 + RAY_DISTANCE;
    const SIDE_VERTICAL_DISTANCE: f32 = 6.0 - RAY_DISTANCE;
    const SIDE_HORIZONTAL_DISTANCE: f32 = 5.5 + RAY_DISTANCE;
    const SIDE_VERTICAL_OFFSET: f32 = 0.3;
    let point1 = Vec2::new(
        stuff.4.translation.x,
        stuff.4.translation.y - BOTTOM_VERTICAL_DISTANCE,
    );
    let point2 = Vec2::new(
        stuff.4.translation.x - 5.5,
        stuff.4.translation.y - BOTTOM_VERTICAL_DISTANCE,
    );
    // point 3 is also underneath but all the way to the right
    let point3 = Vec2::new(
        stuff.4.translation.x + 5.5,
        stuff.4.translation.y - BOTTOM_VERTICAL_DISTANCE,
    );
    // point 4 is between point 1 and point 2
    let point4 = Vec2::new(
        stuff.4.translation.x - 2.75,
        stuff.4.translation.y - BOTTOM_VERTICAL_DISTANCE,
    );
    // point 5 is between point 1 and point 3
    let point5 = Vec2::new(
        stuff.4.translation.x + 2.75,
        stuff.4.translation.y - BOTTOM_VERTICAL_DISTANCE,
    );
    // now we have point 6 which is at 5.99 y and 5.6 x, on the side
    let point6 = Vec2::new(
        stuff.4.translation.x + SIDE_HORIZONTAL_DISTANCE,
        stuff.4.translation.y - SIDE_VERTICAL_DISTANCE + SIDE_VERTICAL_OFFSET,
    );
    // now we have point 7 which is at 5.99 y and -5.6 x, on the side
    let point7 = Vec2::new(
        stuff.4.translation.x - SIDE_HORIZONTAL_DISTANCE,
        stuff.4.translation.y - SIDE_VERTICAL_DISTANCE + SIDE_VERTICAL_OFFSET,
    );
    let down = Vec2::new(0.0, -1.0);
    let left = Vec2::new(-1.0, 0.0);
    let right = Vec2::new(1.0, 0.0);
    let max_toi = 0.2;

    // gizmo each one as a line
    gizmos.line_2d(point1, Vec2::new(point1.x, point1.y - max_toi), Color::RED);
    gizmos.line_2d(point2, Vec2::new(point2.x, point2.y - max_toi), Color::RED);
    gizmos.line_2d(point3, Vec2::new(point3.x, point3.y - max_toi), Color::RED);
    gizmos.line_2d(point4, Vec2::new(point4.x, point4.y - max_toi), Color::RED);
    gizmos.line_2d(point5, Vec2::new(point5.x, point5.y - max_toi), Color::RED);
    gizmos.line_2d(point6, Vec2::new(point6.x + max_toi, point6.y), Color::RED);
    gizmos.line_2d(point7, Vec2::new(point7.x - max_toi, point7.y), Color::RED);

    // we can check multiple at once with `.any_*`
    if keys.any_pressed([KeyCode::W, KeyCode::Up]) {
        // time for Ground Check! omg!
        let mut ground_check = false;

        // raycast each point in if else ifs
        let solid = true;
        let filter = QueryFilter::default();

        if let Some((entity, toi)) = rapier_context.cast_ray(point1, down, max_toi, solid, filter) {
            ground_check = true;
        } else if let Some((entity, toi)) =
            rapier_context.cast_ray(point2, down, max_toi, solid, filter)
        {
            ground_check = true;
        } else if let Some((entity, toi)) =
            rapier_context.cast_ray(point3, down, max_toi, solid, filter)
        {
            ground_check = true;
        } else if let Some((entity, toi)) =
            rapier_context.cast_ray(point4, down, max_toi, solid, filter)
        {
            ground_check = true;
        } else if let Some((entity, toi)) =
            rapier_context.cast_ray(point5, down, max_toi, solid, filter)
        {
            ground_check = true;
        } else if let Some((entity, toi)) =
            rapier_context.cast_ray(point6, left, max_toi, solid, filter)
        {
            ground_check = true;
        } else if let Some((entity, toi)) =
            rapier_context.cast_ray(point7, right, max_toi, solid, filter)
        {
            ground_check = true;
        }

        if ground_check {
            stuff.3.linvel.y = stuff.1.jump_force;
        }
    }
    let (weapon, mut weapon_transform, mut weapon_sprite, mut weapon_global_transform, _, _) =
        weapon_query.single_mut();
    if keys.any_pressed([KeyCode::A, KeyCode::Left]) {
        stuff.3.linvel.x = -stuff.1.speed;
        // flip
        stuff.5.flip_x = true;
        weapon_sprite.flip_x = true;
    }
    if keys.any_pressed([KeyCode::S, KeyCode::Down]) {
        stuff.6.force.y = -200.0;
    } else {
        stuff.6.force.y = 0.0;
    }
    if keys.any_pressed([KeyCode::D, KeyCode::Right]) {
        stuff.3.linvel.x = stuff.1.speed;
        // flip
        stuff.5.flip_x = false;
        weapon_sprite.flip_x = false;
    }

    // + to zoom in
    if keys.pressed(KeyCode::Equals) {
        camera.1.scale *= 1.01;
    }
    // - to zoom out
    if keys.pressed(KeyCode::Minus) {
        camera.1.scale /= 1.01;
    }

    // clamp zoom
    if camera.1.scale < 0.005 {
        camera.1.scale = 0.005;
    }
    if camera.1.scale > 0.2 {
        camera.1.scale = 0.2;
    }
    /*
    // if e just pressed, invert color of all pixels in player sprite
    if keys.just_pressed(KeyCode::E) {
        let mut sprite = stuff.5;
        let mut texture = sprite.clone();
        let mut image = texture.as_mut().unwrap().as_mut().unwrap();
        let mut pixels = image.as_mut();
        for i in 0..pixels.len() {
            if pixels[i] == 0 {
                pixels[i] = 255;
            } else {
                pixels[i] = 0;
            }
        }
        sprite.texture = texture;
    }*/

    // smoothly center camera on player
    //camera.2.translation = Vec3::new(stuff.4.translation.x, stuff.4.translation.y, 0.0);
    let target = Vec3::new(stuff.4.translation.x, stuff.4.translation.y, 0.0);
    camera.2.translation = camera.2.translation.lerp(target, 0.1);

    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.4.viewport_to_world(camera.3, cursor))
        .map(|ray| ray.origin.truncate())
    {
        // make weapon look at world_position
        let global_pos = weapon_global_transform.translation();
        weapon_transform.rotation = Quat::from_rotation_z(
            (world_position.y - global_pos.y).atan2(world_position.x - global_pos.x),
        );
        // if flip_x is true on it, spin it 180 degrees
        if weapon_sprite.flip_x {
            weapon_transform.rotation *= Quat::from_rotation_z(std::f32::consts::PI);
        }

        if buttons.pressed(MouseButton::Left) {
            // Left button was pressed, lets spawn cube at mouse
            commands.spawn((
                Collider::cuboid(2.0, 2.0),
                RigidBody::Dynamic,
                TransformBundle::from(Transform::from_xyz(world_position.x, world_position.y, 0.0)),
            ));
        }
    }
}
/*
// it modifies the texture resource
fn modify_sprite_test(
    mut query: Query<(&Character, &mut Handle<Image>, Without<Player>)>,
    mut res: ResMut<Assets<Image>>,
) {
    // we need to set the first pixel to something random!!11
    // lets get the image, its cone.png
    let image = query.single_mut().1;
    let mut image = res.get_mut(image.id()).unwrap();
    image.data[0] = rand::random::<u8>(); // the red channel
    image.data[1] = rand::random::<u8>(); // the green channel
    image.data[2] = rand::random::<u8>(); // the blue channel
                                          // alpha always 1
    image.data[3] = 255;
}
*/
