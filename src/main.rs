use std::ops::Sub;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::Material2d;
use bevy::window::PresentMode;
use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    sprite::MaterialMesh2dBundle,
    window::PrimaryWindow,
};
use bevy_egui::egui::RichText;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::rapier::dynamics::{RigidBodyHandle, RigidBodySet};
use bevy_turborand::prelude::*;

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

#[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
pub struct MatterMaterial {
    #[uniform(0)]
    color: Color,
    #[uniform(1)]
    strokeColor: Color,
    #[uniform(2)]
    strokeWidth: f32,
    #[texture(3)]
    #[sampler(4)]
    color_texture: Handle<Image>,
}

// All functions on `Material2d` have default impls. You only need to implement the
// functions that are relevant for your material.
impl Material2d for MatterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/matter.wgsl".into()
    }
}

#[derive(Debug, Clone, Copy, SystemSet, PartialEq, Eq, Hash)]
pub struct EguiUnfocusedSystemSet;

// enum of all the tools, we will use it in a resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tool {
    Drag,
    Rectangle,
    Circle,
    Test,
}

#[derive(Resource)]
struct Tools {
    current_tool: Tool,
}

#[derive(Resource)]
struct UIState {
    closed_welcome: bool,
}

#[derive(Component)]
struct DrawingRectangle {
    start: Vec2,
}

#[derive(Component)]
struct DrawingCircle {
    start: Vec2,
}

#[derive(Resource, Deref, DerefMut, PartialEq, Eq, Default)]
struct EguiWantsFocus(bool);

fn check_egui_wants_focus(
    mut contexts: Query<&mut bevy_egui::EguiContext>,
    mut wants_focus: ResMut<EguiWantsFocus>,
) {
    let ctx = contexts.iter_mut().next();
    let new_wants_focus = if let Some(ctx) = ctx {
        let ctx = ctx.into_inner().get_mut();
        ctx.wants_pointer_input() || ctx.wants_keyboard_input()
    } else {
        false
    };
    wants_focus.set_if_neq(EguiWantsFocus(new_wants_focus));
}

fn main() {
    let mut app = App::new();
    app.insert_resource(Msaa::Sample4)
        .insert_resource(ClearColor(Color::rgb(
            0.13333333333333333,
            0.11764705882352941,
            0.2901960784313726,
        )))
        .insert_resource(UIState {
            closed_welcome: false,
        })
        .add_plugins(
            (EmbeddedAssetPlugin {
                mode: PluginMode::ReplaceDefault,
            }),
        )
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resizable: true,
                title: "Simulo".to_string(),
                mode: bevy::window::WindowMode::Windowed,
                #[cfg(target_arch = "wasm32")]
                present_mode: PresentMode::default(), // wasm32-unknown-unknown doesn't support PresentMode::Immediate
                // on everything other than wasm32-unknown-unknown, immediate is used up
                #[cfg(not(target_arch = "wasm32"))]
                present_mode: PresentMode::Immediate,
                fit_canvas_to_parent: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(RngPlugin::default())
        .add_plugins(EguiPlugin)
        .add_plugins(ShapePlugin)
        .add_plugins(PanCamPlugin::default())
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(12.0))
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Update, simulate_springs)
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, ui_system)
        .add_systems(Update, keyboard_input.in_set(EguiUnfocusedSystemSet));

    app.init_resource::<EguiWantsFocus>()
        .add_systems(PostUpdate, check_egui_wants_focus)
        .configure_sets(
            Update,
            EguiUnfocusedSystemSet.run_if(resource_equals(EguiWantsFocus(false))),
        );

    // matter material
    app.init_asset::<MatterMaterial>();

    app.run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<MatterMaterial>>,
) {
    commands.insert_resource(Tools {
        current_tool: Tool::Drag,
    });
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

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    #[cfg(target_arch = "wasm32")]
    let platform_info =
        "Browser - Note: Performance is much better on desktop/mobile \"native\" builds";
    #[cfg(not(target_arch = "wasm32"))]
    let platform_info = "Native";
    let package_name = format!("Simulo Alpha v{} - {}", VERSION, platform_info);

    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            package_name,
            TextStyle {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/Urbanist-SemiBold.ttf"),
                font_size: 20.0,
                ..default()
            },
        ) // Set the alignment of the Text
        .with_text_alignment(TextAlignment::Center)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        }),
    ));

    // 1000 rigidbody boxes stacked on Y axis
    /*for i in 0..50 {
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
    }*/
    /*
    commands.spawn(MaterialMesh2dBundle {
        material: materials.add(MatterMaterial {
            color: Color::RED,
            strokeColor: Color::BLACK,
            strokeWidth: 1.0,
            color_texture: asset_server.load("icon_square.png"),
        }),
        transform: Transform::from_translation(Vec3::new(0., 10., 0.)),
        ..Default::default()
    });*/
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

fn ui_system(
    mut contexts: EguiContexts,
    mut tool_res: ResMut<Tools>,
    mut ui_state: ResMut<UIState>,
) {
    if !ui_state.closed_welcome {
        egui::Window::new("Welcome to the new Simulo!").show(contexts.ctx_mut(), |ui| {
        ui.label("This is the brand new Rust Simulo. It's pretty nice maybe.\n");

        let list_spacing: f32 = 5.;

        ui.label(RichText::new("A few things to note:").underline());
        ui.add_space(list_spacing);

        ui.label(RichText::new(" - This is insanely early in development, don't expect much.").strong());
        ui.add_space(list_spacing);

        #[cfg(target_arch = "wasm32")]
        ui.label(RichText::new(" - Performance is much better on desktop/mobile \"native\" builds, you're on the web version.").strong().color(egui::Color32::GOLD));
        #[cfg(not(target_arch = "wasm32"))]
        ui.label(" - You're running a native build, performance should be good.");

        ui.add_space(list_spacing);

        ui.label(" - Vsync is intentionally disabled for now so there's less latency, but you might get screen tearing.");
        ui.add_space(list_spacing);

        ui.label(" - The UI will have a theme soon, right now this is just the default egui theme.");
        ui.add_space(list_spacing);

        ui.label(" - Middle click to pan, scroll to zoom. Currently it lets you right click to pan, but this will be changed when the right click menu is added, so don't get used to it.");
        ui.add_space(list_spacing);

        ui.separator();

        ui.horizontal(|ui| {
            // Dismiss
            if ui.button("Dismiss").clicked() {
                // Close the window
                ui_state.closed_welcome = true;
            }

            ui.add(egui::Hyperlink::from_label_and_url(
                "Join Simulo Discord server",
                "https://discord.gg/YRspMMj8HR",
            ));

            ui.add(egui::Hyperlink::from_label_and_url(
                "Source code",
                "https://github.com/Carroted/simulo_bevy",
            ));
        });
    });
    }
    // tool radio buttons
    egui::Window::new("Tools").show(contexts.ctx_mut(), |ui| {
        ui.radio_value(&mut tool_res.current_tool, Tool::Drag, "Drag");
        ui.radio_value(&mut tool_res.current_tool, Tool::Rectangle, "Rectangle");
        ui.radio_value(&mut tool_res.current_tool, Tool::Circle, "Circle");
        ui.radio_value(&mut tool_res.current_tool, Tool::Test, "Test");
    });
}

fn spawn_person(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    color: Color,
    world_position: Vec2,
) {
    let body = commands
        .spawn((
            RigidBody::Dynamic,
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(6., 2.09941520468 * 6.)),
                    color,
                    ..default()
                },
                texture: asset_server.load("body.png"),
                transform: Transform::from_translation(Vec3::new(
                    world_position.x,
                    world_position.y,
                    0.,
                )),
                ..default()
            },
        ))
        .with_children(|children| {
            children.spawn((
                Collider::round_cuboid(2.4, 2.1, 0.04),
                Transform::from_translation(Vec3::new(0., -3.7, 0.)),
            ));
            children.spawn((
                Collider::ball(2.9),
                Transform::from_translation(Vec3::new(0., -1.8, 0.)),
            ));
        })
        .id();

    let joint = RevoluteJointBuilder::new()
        .local_anchor1(Vec2::new(0.0, 1.8)) // anchor on body
        .local_anchor2(Vec2::new(0.0, -2.5)); // anchor on head

    // now the head, its circle of same radius 2.9. no head.png, we just circle.png like normal
    commands.spawn((
        RigidBody::Dynamic,
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(2.9 * 2., 2.9 * 2.)),
                color,
                ..default()
            },
            texture: asset_server.load("circle.png"),
            transform: Transform::from_translation(Vec3::new(
                world_position.x,
                world_position.y + 2.09941520468 * 6. / 2. + 2.9 / 2.,
                0.,
            )),
            ..default()
        },
        Collider::ball(2.9),
        ImpulseJoint::new(body, joint),
    ));
}

fn keyboard_input(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    rapier_context: Res<RapierContext>,
    mut rapier_config: ResMut<RapierConfiguration>,
    mut gizmos: Gizmos,
    mut camera_query: Query<(
        &MainCamera,
        &mut OrthographicProjection,
        &mut Transform,
        &GlobalTransform,
        &Camera,
        Without<Player>,
    )>,
    transforms_query: Query<(&Transform, Without<MainCamera>, &RigidBody)>,
    buttons: Res<Input<MouseButton>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut world_spring_query: Query<(Entity, &mut WorldSpring)>,
    mut tool_res: ResMut<Tools>,
    mut drawing_rectangle_query: Query<(
        &DrawingRectangle,
        &mut Sprite,
        Entity,
        &mut Transform,
        Without<WorldSpring>,
        Without<MultiBodySpring>,
        Without<MainCamera>,
        Without<RigidBody>,
    )>,
    mut drawing_circle_query: Query<(
        &DrawingCircle,
        &mut Sprite,
        Entity,
        &mut Transform,
        Without<WorldSpring>,
        Without<MultiBodySpring>,
        Without<MainCamera>,
        Without<RigidBody>,
        Without<DrawingRectangle>,
    )>,
    mut global_rng: ResMut<GlobalRng>,
    // asset server real
    asset_server: Res<AssetServer>,
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

    if keys.just_pressed(KeyCode::Key1) {
        tool_res.current_tool = Tool::Drag;
    } else if keys.just_pressed(KeyCode::Key2) {
        tool_res.current_tool = Tool::Rectangle;
    }

    if keys.just_pressed(KeyCode::Space) {
        rapier_config.physics_pipeline_active = !rapier_config.physics_pipeline_active;
    }

    let current_tool = tool_res.current_tool;

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
        // e to spawn a person real
        if keys.just_pressed(KeyCode::P) {
            spawn_person(
                &mut commands,
                &asset_server,
                Color::rgb(133. / 255., 243. / 255., 112. / 255.),
                world_position,
            );
        }
        if keys.pressed(KeyCode::V) {
            let mut color = Color::rgb(1., 1., 1.);
            if keys.pressed(KeyCode::ShiftLeft) {
                color = Color::rgb(0.5, 0.5, 1.);
            }
            gizmos.rect_2d(world_position, 0.0, Vec2::new(8., 16.), color);
        }
        if keys.pressed(KeyCode::H) {
            let mut color = Color::rgb(1., 1., 1.);
            if keys.pressed(KeyCode::ShiftLeft) {
                color = Color::rgb(0.5, 0.5, 1.);
            }
            gizmos.rect_2d(world_position, 0.0, Vec2::new(16., 8.), color);
        }
        if keys.just_released(KeyCode::V) {
            let mut color = Color::rgb(1., 1., 1.);
            if keys.pressed(KeyCode::ShiftLeft) {
                color = Color::rgb(0.5, 0.5, 1.);
            }
            let mut ent = commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(8., 16.)),
                        color,
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        world_position.x,
                        world_position.y,
                        0.,
                    )),

                    ..default()
                },
                Collider::cuboid(4., 8.),
            ));
            if !keys.pressed(KeyCode::ShiftLeft) {
                ent.insert(RigidBody::Dynamic);
            }
        }
        if keys.just_released(KeyCode::H) {
            let mut color = Color::rgb(1., 1., 1.);
            if keys.pressed(KeyCode::ShiftLeft) {
                color = Color::rgb(0.5, 0.5, 1.);
            }
            let mut ent = commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(16., 8.)),
                        color,
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        world_position.x,
                        world_position.y,
                        0.,
                    )),

                    ..default()
                },
                Collider::cuboid(8., 4.),
            ));
            if !keys.pressed(KeyCode::ShiftLeft) {
                ent.insert(RigidBody::Dynamic);
            }
        }
        if keys.just_pressed(KeyCode::M) {
            spawn_person(
                &mut commands,
                &asset_server,
                Color::rgb(232. / 255., 80. / 255., 74. / 255.),
                world_position,
            );
        }
        if buttons.pressed(MouseButton::Left) {
            if current_tool == Tool::Rectangle {
                // Left button was pressed, lets spawn cube at mouse
                /*commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.75, 0.25, 0.25),
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
                    ExternalImpulse {
                        impulse: Vec2::new(0., 30.),
                        ..Default::default()
                    },
                ));*/
            }
            // if its test, spam cubes
            if current_tool == Tool::Test {
                for _ in 0..5 {
                    commands.spawn((
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgb(0.75, 0.25, 0.25),
                                custom_size: Some(Vec2::new(4., 4.)),
                                ..default()
                            },
                            transform: Transform::from_translation(Vec3::new(
                                world_position.x + global_rng.f32() * 30. - 15.,
                                world_position.y + global_rng.f32() * 30. - 15.,
                                0.00,
                            )),
                            ..default()
                        },
                        Collider::cuboid(2.0, 2.0),
                        RigidBody::Dynamic,
                    ));
                }
            }
        }
        if buttons.just_released(MouseButton::Left) {
            if current_tool == Tool::Rectangle {
                // query time
                let (drawing_rectangle, mut sprite, entity, mut transform, _, _, _, _) =
                    drawing_rectangle_query.single_mut();
                let start = drawing_rectangle.start;
                let end = world_position;
                let width = (start.x - end.x).abs();
                let height = (start.y - end.y).abs();
                let size = Vec2::new(width, height);
                let center = (start + end) / 2.;
                sprite.custom_size = Some(size);
                // same color but alpha 1
                sprite.color =
                    Color::rgba(sprite.color.r(), sprite.color.g(), sprite.color.b(), 1.);
                let mut ent = commands.get_entity(entity).unwrap();
                ent.remove::<DrawingRectangle>();
                ent.insert((
                    Collider::cuboid(width / 2., height / 2.),
                    RigidBody::Dynamic,
                ));
                // transform it up
                transform.translation = Vec3::new(center.x, center.y, 0.);
                ent.remove::<Aabb>(); // force recalculation
            }
            // the the the
            if current_tool == Tool::Circle {
                let (drawing_circle, mut sprite, entity, mut transform, _, _, _, _, _) =
                    drawing_circle_query.single_mut();

                let start = drawing_circle.start;
                let end = world_position;
                let width = (start.x - end.x);
                let height = (start.y - end.y);
                let size = width.abs().max(height);
                let mut center = (start + Vec2::new(size, size));

                sprite.custom_size = Some(Vec2::new(size / 2., size / 2.));
                // same color but alpha 1
                sprite.color =
                    Color::rgba(sprite.color.r(), sprite.color.g(), sprite.color.b(), 1.);
                let mut ent = commands.get_entity(entity).unwrap();
                ent.remove::<DrawingRectangle>();
                ent.insert((Collider::ball(size / 2.), RigidBody::Dynamic));
                // transform it up
                transform.translation = Vec3::new(center.x, center.y, 0.);
                ent.remove::<Aabb>(); // force recalculation
            }
        }
        if buttons.just_pressed(MouseButton::Left) {
            if (current_tool == Tool::Rectangle) {
                // spawn just a display of a transparent rectangle with 0 size, no collider or rb or anything, when mouse moves, update the size, when mouse is released, spawn the actual thing
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(
                                global_rng.f32(),
                                global_rng.f32(),
                                global_rng.f32(),
                                0.5,
                            ),
                            custom_size: Some(Vec2::new(0., 0.)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(
                            world_position.x,
                            world_position.y,
                            0.00,
                        )),
                        ..default()
                    },
                    DrawingRectangle {
                        start: world_position,
                    },
                ));
            }
            if (current_tool == Tool::Circle) {
                // spawn just a display of a transparent circle with 0 size, no collider or rb or anything, when mouse moves, update the size, when mouse is released, spawn the actual thing
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgba(
                                global_rng.f32(),
                                global_rng.f32(),
                                global_rng.f32(),
                                0.5,
                            ),
                            custom_size: Some(Vec2::new(0., 0.)),
                            ..default()
                        },
                        texture: asset_server.load("circle.png"),
                        transform: Transform::from_translation(Vec3::new(
                            world_position.x,
                            world_position.y,
                            0.00,
                        )),
                        ..default()
                    },
                    DrawingCircle {
                        start: world_position,
                    },
                ));
            }
            if current_tool == Tool::Drag {
                let solid = true;
                let filter = QueryFilter::default();
                if let Some((entity, projection)) =
                    rapier_context.project_point(world_position, solid, filter)
                {
                    let result = transforms_query.get(entity);
                    // check if its real
                    if result.is_ok() {
                        let (transform, _, _) = result.unwrap();
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
                        stiffness: 0.04,
                    },
                            Sleeping {
                                sleeping: false,
                                ..Default::default()
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
        // query time
        /*let (drawing_rectangle, mut sprite, _, mut transform, _, _, _, _) =
            drawing_rectangle_query
        let start = drawing_rectangle.start;
        let end = world_position;
        let width = (start.x - end.x).abs();
        let height = (start.y - end.y).abs();
        let size = Vec2::new(width, height);
        let center = (start + end) / 2.;
        sprite.custom_size = Some(size);
        // transform it up
        transform.translation = Vec3::new(center.x, center.y, 0.);*/
        for (drawing_rectangle, mut sprite, entity, mut transform, _, _, _, _) in
            drawing_rectangle_query.iter_mut()
        {
            let start = drawing_rectangle.start;
            let end = world_position;
            let width = (start.x - end.x).abs();
            let height = (start.y - end.y).abs();
            let size = Vec2::new(width, height);
            let center = (start + end) / 2.;
            sprite.custom_size = Some(size);
            // transform it up
            transform.translation = Vec3::new(center.x, center.y, 0.);

            let mut ent = commands.get_entity(entity).unwrap();

            ent.remove::<Aabb>(); // force recalculation so it doesnt cull incorrectly (size starts at 0, if we dont do this it will be culled when center is outside of the screen)
        }

        for (drawing_circle, mut sprite, entity, mut transform, _, _, _, _, _) in
            drawing_circle_query.iter_mut()
        {
            let start = drawing_circle.start;
            let end = world_position;
            let width = (start.x - end.x).abs();
            let height = (start.y - end.y).abs();
            let size = width.max(height);
            let center = (start + end) / 2.;
            sprite.custom_size = Some(Vec2::new(size, size));
            // transform it up
            transform.translation = Vec3::new(center.x, center.y, 0.);

            let mut ent = commands.get_entity(entity).unwrap();

            ent.remove::<Aabb>(); // force recalculation so it doesnt cull incorrectly (size starts at 0, if we dont do this it will be culled when center is outside of the screen)
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
        /*
        let com = global_transform
            .transform_point(mass_props.local_center_of_mass.extend(0.))
            .truncate();
        gizmos.line_2d(
            com + Vec2::new(0., 0.8),
            com - Vec2::new(0., 0.8),
            Color::GREEN,
        );
        gizmos.line_2d(
            com + Vec2::new(0.8, 0.),
            com - Vec2::new(0.8, 0.),
            Color::GREEN,
        );

        gizmos.circle_2d(point_a_world, 0.8, Color::RED);

        gizmos.circle_2d(point_b_world, 0.8, Color::BLUE);*/

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
