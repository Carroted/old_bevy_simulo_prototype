use std::ops::Sub;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::window::PresentMode;
use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    sprite::MaterialMesh2dBundle,
    window::PrimaryWindow,
};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::rapier::dynamics::{RigidBodyHandle, RigidBodySet};
use rand;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct MultiBodySpring {
    body_b_rb: RigidBodyHandle,
    local_anchor_a: Vec2,
    local_anchor_b: Vec2,
    stiffness: f32,
    damping: f32,
    target_len: f32,
}

#[derive(Component)]
struct WorldSpring {
    local_anchor_a: Vec2,
    world_anchor_b: Vec2,
    stiffness: f32,
    damping: f32,
    target_len: f32,
}

fn main() {
    App::new()
        .insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::rgb(
            0.13333333333333333,
            0.11764705882352941,
            0.2901960784313726,
        )))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resizable: true,
                title: "Simulo".to_string(),
                mode: bevy::window::WindowMode::Windowed,
                #[cfg(target_arch = "wasm32")]
                present_mode: PresentMode::default(),
                // on everything other than wasm32-unknown-unknown, immediate is used up
                #[cfg(not(target_arch = "wasm32"))]
                present_mode: PresentMode::Immediate,
                fit_canvas_to_parent: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(ShapePlugin)
        .add_plugins(PanCamPlugin::default())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(12.0))
        .add_systems(Update, simulate_springs)
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_input)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Camera2dBundle {
                camera: Camera { ..default() },
                ..default()
            },
            MainCamera,
        ))
        .insert(PanCam {
            grab_buttons: vec![MouseButton::Right, MouseButton::Middle],
            ..Default::default()
        });

    /* Create the ground. */
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.6313725490196078, 0.6745098039215687, 0.9803921568627451),
                custom_size: Some(Vec2::new(10000.0, 1000.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0., -1000., 0.)),
            ..default()
        },
        Collider::cuboid(5000.0, 500.0),
    ));

    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            #[cfg(target_arch = "wasm32")]
            "Simulo\nBrowser (WASM)\n",
            #[cfg(not(target_arch = "wasm32"))]
            "Simulo\nNative\n",
            TextStyle {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/Urbanist-SemiBold.ttf"),
                font_size: 100.0,
                ..default()
            },
        ) // Set the alignment of the Text
        .with_text_alignment(TextAlignment::Center)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(5.0),
            ..default()
        }),
        ColorText,
    ));

    // 1000 rigidbody boxes stacked on Y axis
    for i in 0..50 {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(0.25, 0.25, 0.75),
                    custom_size: Some(Vec2::new(16., 4.)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(140., 1. + i as f32 * 4.1, 0.00)),
                ..default()
            },
            Collider::cuboid(8.0, 2.0),
            RigidBody::Dynamic,
        ));
    }
}

/*     getLocalPoint(bodyPosition: RAPIER.Vector2, bodyRotation: number, worldPoint: RAPIER.Vector2) {
    const cos = Math.cos(bodyRotation);
    const sin = Math.sin(bodyRotation);
    const x = worldPoint.x - bodyPosition.x;
    const y = worldPoint.y - bodyPosition.y;
    const localX = x * cos + y * sin;
    const localY = -x * sin + y * cos;
    return new RAPIER.Vector2(localX, localY);
} */

fn get_local_point(body_position: Vec2, body_rotation: f32, world_point: Vec2) -> Vec2 {
    let cos = body_rotation.cos();
    let sin = body_rotation.sin();
    let x = world_point.x - body_position.x;
    let y = world_point.y - body_position.y;
    let local_x = x * cos + y * sin;
    let local_y = -x * sin + y * cos;
    Vec2::new(local_x, local_y)
}

fn keyboard_input(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    rapier_context: Res<RapierContext>,
    mut gizmos: Gizmos,
    mut camera_query: Query<(
        &MainCamera,
        &mut OrthographicProjection,
        &mut Transform,
        &GlobalTransform,
        &Camera,
        Without<Player>,
    )>,
    transforms_query: Query<(&Transform, Without<MainCamera>)>,
    buttons: Res<Input<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut world_spring_query: Query<(Entity, &mut WorldSpring)>,
) {
    // There is only one primary window, so we can similarly get it from the query:
    let window = q_window.single();

    // get camera
    let mut camera: (
        &MainCamera,
        Mut<'_, OrthographicProjection>,
        Mut<'_, Transform>,
        &GlobalTransform,
        &Camera,
        _,
    ) = camera_query.single_mut();

    if buttons.just_released(MouseButton::Left) {
        // remove all springs
        for (entity, mut spring) in world_spring_query.iter_mut() {
            commands.entity(entity).remove::<WorldSpring>();
        }
    }
    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.4.viewport_to_world(camera.3, cursor))
        .map(|ray| ray.origin.truncate())
    {
        for (_, mut spring) in world_spring_query.iter_mut() {
            spring.world_anchor_b = world_position;
        }
        if buttons.just_pressed(MouseButton::Left) {
            // Left button was pressed, lets spawn cube at mouse
            /*commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.25, 0.25, 0.75),
                        custom_size: Some(Vec2::new(4., 4.)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        world_position.x,
                        world_position.y,
                        0.00,
                    )),
                    ..default()
                },
                Collider::cuboid(2.0, 2.0),
                RigidBody::Dynamic,
            ));*/
            let solid = true;
            let filter = QueryFilter::default();
            if let Some((entity, projection)) =
                rapier_context.project_point(world_position, solid, filter)
            {
                let (transform, _) = transforms_query.get(entity).unwrap();
                let mut ent = commands.get_entity(entity).unwrap();
                ent.insert((
                    ExternalImpulse::default(),
                    Velocity::default(),
                    GlobalTransform::default(),
                    ReadMassProperties::default(),
                    WorldSpring {
                        target_len: 0.,
                        damping: 0.,
                        local_anchor_a: /*transform
                            .transform_point(world_position.extend(0.))
                            .truncate(), // Vec2::new(10., 0.),*/
                            get_local_point(
                                transform.translation.truncate(),
                                transform.rotation.to_euler(EulerRot::XYZ).2,
                                world_position,
                            ),
                        world_anchor_b: world_position,
                        stiffness: 0.02,
                    },
                ));

                // The collider closest to the point has this `handle`.
                println!(
                    "Projected point on entity {:?}. Point projection: {}",
                    entity, projection.point
                );
                println!(
                    "Point was inside of the collider shape: {}",
                    projection.is_inside
                );
                println!("Springed up!");
            }
        }
    }
}

fn gcross(vec_a: Vec2, vec_b: Vec2) -> f32 {
    vec_a.x * vec_b.y - vec_a.y * vec_b.x
}

fn simulate_springs(
    mut multibody_spring_query: Query<(
        &mut MultiBodySpring,
        &ExternalImpulse,
        Without<WorldSpring>,
    )>,
    mut world_spring_query: Query<(
        &WorldSpring,
        &Velocity,
        &GlobalTransform,
        &mut ExternalImpulse,
        Without<MultiBodySpring>,
        &ReadMassProperties,
    )>,
    mut other_impulse_query: Query<(
        &mut ExternalImpulse,
        Without<MultiBodySpring>,
        Without<WorldSpring>,
    )>,
    rapier_context: Res<RapierContext>,
    mut gizmos: Gizmos,
) {
    // iterate over all springs
    for (mut spring, rigidbody_a_impulse, _) in multibody_spring_query.iter_mut() {
        // get the other impulser of the spring
        let entity = rapier_context.rigid_body_entity(spring.body_b_rb).unwrap();
        let mut rigidbody_b_stuff = other_impulse_query.get_mut(entity).unwrap();
        let rigidbody_b_impulse = rigidbody_b_stuff.0.as_mut();
        {
            // Apply an impulse of (10, 10) to the rigidbody
            rigidbody_b_impulse.impulse.x = 10.;
            rigidbody_b_impulse.impulse.y = 10.;
        }
    }

    // world ones
    for (spring, velocity, global_transform, mut rigidbody_impulse, _, mass_props) in
        world_spring_query.iter_mut()
    {
        let point_a_world = global_transform
            .transform_point(spring.local_anchor_a.extend(0.))
            .truncate();
        let point_b_world = spring.world_anchor_b;

        gizmos.line_2d(point_a_world, point_b_world, Color::WHITE);

        let linvel_a = velocity.linvel;
        let linvel_b = Vec2::new(0., 0.);
        let angvel_a = velocity.angvel;

        let spring_vector = point_b_world - point_a_world;
        let direction = spring_vector.normalize();
        let distance = (spring_vector.x.powf(2.) + spring_vector.y.powf(2.)).sqrt();

        // make sure the distance is greater than certain threshold
        if distance < 0.001 {
            continue;
        }

        // The spring code is based on what was used in Simulo NT:
        /* // Compute relative velocity of the anchor points, u
        const u = this.sub(velB, velA);
        const rj = this.crossZV(spring.getBodyBAngularVelocity(), spring.localAnchorB);
        const ri = this.crossZV(spring.getBodyAAngularVelocity(), spring.localAnchorA);
        const tmp = this.add(u, rj, ri);
        const f = this.multiply(direction, -spring.stiffness * (distance - spring.targetLength) - spring.damping * this.dot(u, direction));
        const forceA = this.multiply(f, -1);
        const forceB = f; */

        let ri = Vec2::new(
            -angvel_a * spring.local_anchor_a.y,
            angvel_a * spring.local_anchor_a.x,
        );
        let u = linvel_b - linvel_a + ri;
        let f = direction
            * ((-spring.stiffness * (distance - spring.target_len))
                - (spring.damping * u.dot(direction)));

        let force_a = f * -1.;
        let force_b = f;

        gizmos.circle_2d(
            global_transform
                .transform_point(mass_props.local_center_of_mass.extend(0.))
                .truncate(),
            1.,
            Color::GREEN,
        );

        gizmos.circle_2d(point_a_world, 1., Color::RED);

        gizmos.circle_2d(point_b_world, 1., Color::BLUE);

        // Figure out what impulse would be on the body if applied at certain point
        /*let new_impulse = ExternalImpulse::at_point(
            force_a,
            point_a_world,
            global_transform
                .transform_point(mass_props.local_center_of_mass.extend(0.))
                .truncate(),
        );*/

        // Apply spring force
        rigidbody_impulse.impulse = force_a;
        rigidbody_impulse.torque_impulse = gcross(
            point_a_world
                - global_transform
                    .transform_point(mass_props.local_center_of_mass.extend(0.))
                    .truncate(),
            force_a,
        ) / 250.;
    }
}
