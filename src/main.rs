use bevy::prelude::*;
use bevy::window::PrimaryWindow;

const MAP_WIDTH: i32 = 32;
const MAP_HEIGHT: i32 = 32;
const TILE_SIZE: f32 = 32.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.05, 0.05, 0.08)))
        .init_resource::<CityStats>()
        .insert_resource(SimTimer(Timer::from_seconds(
            0.5,
            TimerMode::Repeating,
        )))
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy City Sim".to_string(),
                    resolution: (1280.0, 720.0).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_systems(Startup, (setup_camera, spawn_map, setup_ui))
        .add_systems(
            Update,
            (
                handle_mouse_input,
                simulation_step,
                update_stats_ui,
            ),
        )
        .run();
}

/// Grid coordinate for each tile.
#[derive(Component)]
struct TileCoord {
    coord: IVec2,
}

/// Zone type of a tile (what the player builds).
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Empty,
    Road,
    Residential,
    Commercial,
    Industrial,
}

/// Per-tile simulation data (simple for now).
#[derive(Component)]
struct TileData {
    population: u32,
    jobs: u32,
}

impl Zone {
    fn next(self) -> Self {
        use Zone::*;
        match self {
            Empty => Road,
            Road => Residential,
            Residential => Commercial,
            Commercial => Industrial,
            Industrial => Empty,
        }
    }

    fn color(self) -> Color {
        use Zone::*;
        match self {
            Empty => Color::srgb(0.1, 0.4, 0.1),       // grass
            Road => Color::srgb(0.2, 0.2, 0.2),        // dark gray
            Residential => Color::srgb(0.1, 0.5, 0.9), // blue
            Commercial => Color::srgb(0.1, 0.8, 0.8),  // teal
            Industrial => Color::srgb(0.8, 0.7, 0.1),  // yellow
        }
    }
}

/// Aggregate city statistics.
#[derive(Resource, Default)]
struct CityStats {
    population: u32,
    jobs: u32,
    money: i64,
}

/// Timer that ticks the simulation.
#[derive(Resource)]
struct SimTimer(Timer);

/// Marker on the UI text that shows stats.
#[derive(Component)]
struct StatsText;

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_map(mut commands: Commands) {
    // Center map around (0, 0)
    let origin_x =
        -(MAP_WIDTH as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0;
    let origin_y =
        -(MAP_HEIGHT as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0;

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let world_x = origin_x + x as f32 * TILE_SIZE;
            let world_y = origin_y + y as f32 * TILE_SIZE;

            let zone = Zone::Empty;

            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: zone.color(),
                        custom_size: Some(Vec2::splat(TILE_SIZE - 1.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        world_x,
                        world_y,
                        0.0,
                    ),
                    ..default()
                },
                TileCoord {
                    coord: IVec2::new(x, y),
                },
                zone,
                TileData {
                    population: 0,
                    jobs: 0,
                },
            ));
        }
    }
}

fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    commands.spawn((
        TextBundle {
            text: Text::from_sections([TextSection::new(
                "Pop: 0  Jobs: 0  Money: 0",
                TextStyle {
                    font,
                    font_size: 24.0,
                    color: Color::WHITE,
                },
            )]),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        },
        StatsText,
    ));
}

/// Handle left mouse clicks: change the zone of the clicked tile.
fn handle_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut tiles: Query<(
        &TileCoord,
        &mut Zone,
        &mut Sprite,
    )>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let window = if let Ok(w) = windows.get_single() {
        w
    } else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let (camera, cam_transform) = if let Ok(v) = camera_q.get_single() {
        v
    } else {
        return;
    };

    let Some(world_pos) =
        camera.viewport_to_world_2d(cam_transform, cursor_pos)
    else {
        return;
    };

    // Convert world position back to tile coordinates.
    let origin_x =
        -(MAP_WIDTH as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0;
    let origin_y =
        -(MAP_HEIGHT as f32 * TILE_SIZE) / 2.0 + TILE_SIZE / 2.0;

    let tx = ((world_pos.x - origin_x) / TILE_SIZE).floor() as i32;
    let ty = ((world_pos.y - origin_y) / TILE_SIZE).floor() as i32;

    if tx < 0 || ty < 0 || tx >= MAP_WIDTH || ty >= MAP_HEIGHT {
        return;
    }

    // Find the tile at this coordinate.
    for (coord, mut zone, mut sprite) in tiles.iter_mut() {
        if coord.coord.x == tx && coord.coord.y == ty {
            *zone = zone.next();
            sprite.color = zone.color();
            break;
        }
    }
}

/// Simple, very toy simulation step.
/// Every tick:
/// - Residential tiles gain population if next to a road
/// - Commercial/Industrial tiles gain jobs
/// - Money increases based on jobs and population
fn simulation_step(
    time: Res<Time>,
    mut timer: ResMut<SimTimer>,
    mut tiles: Query<(
        &TileCoord,
        &Zone,
        &mut TileData,
    )>,
    mut stats: ResMut<CityStats>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    // Reset stats and re-compute from tiles.
    stats.population = 0;
    stats.jobs = 0;

    // Build a quick lookup for zone by coord.
    use std::collections::HashMap;
    let mut zone_map: HashMap<IVec2, Zone> = HashMap::new();
    for (coord, zone, _) in tiles.iter() {
        zone_map.insert(coord.coord, *zone);
    }

    for (coord, zone, mut data) in tiles.iter_mut() {
        match zone {
            Zone::Residential => {
                let neighbors = [
                    IVec2::new(1, 0),
                    IVec2::new(-1, 0),
                    IVec2::new(0, 1),
                    IVec2::new(0, -1),
                ];
                let mut adjacent_road = false;
                for n in neighbors {
                    if let Some(n_zone) =
                        zone_map.get(&(coord.coord + n))
                    {
                        if *n_zone == Zone::Road {
                            adjacent_road = true;
                            break;
                        }
                    }
                }

                if adjacent_road {
                    data.population =
                        (data.population + 1).min(100);
                }
            }
            Zone::Commercial | Zone::Industrial => {
                data.jobs = (data.jobs + 1).min(100);
            }
            Zone::Road | Zone::Empty => {
                data.population = 0;
                data.jobs = 0;
            }
        }

        stats.population += data.population;
        stats.jobs += data.jobs;
    }

    // Money: simple formula for now.
    stats.money += (stats.jobs as i64 / 5)
        - (stats.population as i64 / 10);
}

fn update_stats_ui(
    stats: Res<CityStats>,
    mut query: Query<&mut Text, With<StatsText>>,
) {
    if !stats.is_changed() {
        return;
    }

    if let Ok(mut text) = query.get_single_mut() {
        text.sections[0].value = format!(
            "Pop: {}  Jobs: {}  Money: {}",
            stats.population, stats.jobs, stats.money
        );
    }
}
