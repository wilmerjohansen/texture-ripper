use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::render::camera::ScalingMode;
use bevy::{
    core::FrameCount,
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowTheme},
};
use rfd::FileDialog;
use std::path::PathBuf;

#[derive(Component)]
struct MyCameraMarker;

// Resource to track if we're currently panning
#[derive(Resource, Default)]
struct PanState {
    is_panning: bool,
    start_cursor_world: Vec2,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "texture-ripper".into(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<PanState>()
        .add_systems(Startup, setup)
        .add_systems(Update, (make_visible, handle_keyboard_input, handle_pan))
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera2dBundle {
            projection: OrthographicProjection {
                // don't forget to set `near` and `far`
                near: -1000.0,
                far: 1000.0,
                // ... any other settings you want to change ...
                ..default()
            },
            ..default()
        },
        MyCameraMarker,
    ));
}

/// At this point the gpu is ready to show the app and make the window visible.
fn make_visible(mut window: Query<&mut Window>, frames: Res<FrameCount>) {
    if frames.0 == 3 {
        window.single_mut().visible = true;
    }
}

fn handle_keyboard_input(
    mut commands: Commands,
    window: Query<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_wheel_input: EventReader<MouseWheel>,
    mut pan_state: ResMut<PanState>,
    mut param_set: ParamSet<(
        Query<(&mut OrthographicProjection, &mut Transform), With<MyCameraMarker>>,
        Query<(&Transform, &Handle<Image>), With<Sprite>>,
    )>,
    assets: Res<Assets<Image>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<MyCameraMarker>>,
) {
    // Zooming
    for event in mouse_wheel_input.read() {
        let mut camera_query = param_set.p0();
        let (mut projection, mut camera_transform) = camera_query.single_mut();
        let window = window.single();

        if let Some(mut selected_pixel_pos) = window.cursor_position() {
            selected_pixel_pos.x -= 400.0;
            selected_pixel_pos.y -= 300.0;

            let zoom_factor = if event.y > 0.0 { 0.8 } else { 1.25 };
            projection.scale *= zoom_factor;

            let zoomed_pixel_pos = selected_pixel_pos * zoom_factor;
            let delta_pixel_zoom = zoomed_pixel_pos - selected_pixel_pos;

            println!("selected_pixel_pos {}", selected_pixel_pos);
            println!("zoomed_pixel_pos {}", zoomed_pixel_pos);

            camera_transform.translation.x += delta_pixel_zoom.x;
            camera_transform.translation.y += delta_pixel_zoom.y;
        }
    }

    // Update pan state based on mouse button
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let window = window.single();
        let (camera, camera_transform) = camera_q.single();

        pan_state.is_panning = true;
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor))
        {
            pan_state.start_cursor_world = world_position;
        }

        // Get the cursor position in world coordinates
        if let Some(cursor_position) = window.cursor_position().and_then(|cursor| {
            camera.viewport_to_world_2d(camera_transform, cursor)
        }) {
            for (transform, image_handle) in param_set.p1().iter() {
                // Calculate the local position on the image
                let local_position = cursor_position - transform.translation.truncate();

                if let Some(image) = assets.get(image_handle) {
                    let image_dimensions = image.size();
                    let image_dimensions: Vec2 = Vec2::new(image_dimensions.x as f32, image_dimensions.y as f32);
                    let scaled_image_dimension = image_dimensions * transform.scale.truncate();
                    let bounding_box = Rect::from_center_size(transform.translation.truncate(), scaled_image_dimension);

                    // Check if the cursor is within the image bounds
                    if bounding_box.contains(cursor_position) {
                        // Calculate the pixel position
                        let pixel_x = (((local_position.x + image_dimensions.x / 2.) / transform.scale.x) * image_dimensions.x as f32) as u32;
                        let pixel_y = (((local_position.y + image_dimensions.y / 2.) / transform.scale.y) * image_dimensions.y as f32) as u32;

                        println!("Clicked on pixel at local: ({}, {})", local_position.x, local_position.y);
                        println!("Clicked on pixel at: ({}, {})", pixel_x, pixel_y);
                    }
                }
            }
        }
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        pan_state.is_panning = false;
    }

    if keyboard_input.just_pressed(KeyCode::KeyO) {
        // Open file dialog when 'O' is pressed
        if let Some(path) = FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            println!("{:?}", path);
            commands.spawn(SpriteBundle {
                texture: asset_server.load(path),
                ..default()
            });
        }
    }
}

fn handle_pan(
    window: Query<&Window, With<PrimaryWindow>>,
    pan_state: ResMut<PanState>,
    mut camera_query: Query<(&Camera, &mut GlobalTransform), With<MyCameraMarker>>,
    mut camera_translation: Query<&mut Transform, With<MyCameraMarker>>,
) {
    if pan_state.is_panning {
        let (camera, camera_transform) = camera_query.single_mut();
        let mut transform = camera_translation.single_mut();
        let window = window.single();

        // Get current cursor position in world coordinates
        if let Some(current_cursor_world) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(&camera_transform, cursor))
        {
            // Calculate the difference between current and start position
            let delta = pan_state.start_cursor_world - current_cursor_world;

            // Move the camera by this difference to maintain the cursor's world position
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
        }
    }
}
