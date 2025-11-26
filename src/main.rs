use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::{fs, io::Write, path::Path};

const MAP_WIDTH: i32 = 32;
const MAP_HEIGHT: i32 = 32;
const TILE_SIZE: f32 = 32.0;

// Kenney Roguelike Modern City pack (CC0)
const SPRITE_ZIP_URL: &str = "https://kenney.nl/content/3-assets/11-roguelike-modern-city/roguelikeCity_magenta.png";
const SPRITE_PATH: &str = "assets/roguelike_city.png";
const SPRITE_ASSET_PATH: &str = "roguelike_city.png"; // Path for Bevy AssetServer (no 'assets/' prefix)

fn main() {
    // Download sprites before starting Bevy
    ensure_sprites_downloaded();

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
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_systems(Startup, (load_sprites, setup_camera, spawn_map, setup_ui).chain())
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

fn ensure_sprites_downloaded() {
    let path = Path::new(SPRITE_PATH);

    // If sprite already exists, do nothing
    if path.exists() {
        println!("✓ Sprite pack already present at {}", SPRITE_PATH);
        return;
    }

    // Ensure assets/ directory exists
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Failed to create assets directory: {e}");
            return;
        }
    }

    println!("Downloading Kenney Roguelike Modern City sprites (CC0)...");
    println!("From: {}", SPRITE_ZIP_URL);

    // Blocking HTTP GET
    let response = match reqwest::blocking::get(SPRITE_ZIP_URL) {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to download sprites: {e}");
            eprintln!("You can manually download from: https://www.kenney.nl/assets/roguelike-modern-city");
            return;
        }
    };

    let bytes = match response.bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read sprite response: {e}");
            return;
        }
    };

    let mut file = match fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create sprite file: {e}");
            return;
        }
    };

    if let Err(e) = file.write_all(&bytes) {
        eprintln!("Failed to write sprite file: {e}");
        return;
    }

    println!("✓ Sprites downloaded successfully to {}", SPRITE_PATH);
}

/// Resource holding sprite atlas info
#[derive(Resource)]
struct CitySprites {
    texture: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl Zone {
    /// Get the sprite index for this zone type from the Kenney tileset
    fn sprite_index(self) -> usize {
        use Zone::*;
        match self {
            Empty => 214,      // grass tile
            Road => 235,       // road tile
            Residential => 8,  // small house
            Commercial => 19,  // shop/store
            Industrial => 53,  // factory/warehouse
        }
    }
}

fn load_sprites(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load(SPRITE_ASSET_PATH);
    
    // Kenney's roguelike city sheet is 17x17 tiles at 16x16 pixels each
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(16),  // tile size in pixels
        17,                // columns
        17,                // rows
        None,              // padding
        None,              // offset
    );
    
    let layout_handle = texture_atlases.add(layout);
    
    commands.insert_resource(CitySprites {
        texture,
        layout: layout_handle,
    });
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
    commands.spawn(Camera2d);
}

fn spawn_map(mut commands: Commands, sprites: Res<CitySprites>) {
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
                Sprite {
                    image: sprites.texture.clone(),
                    custom_size: Some(Vec2::splat(TILE_SIZE)),
                    texture_atlas: Some(TextureAtlas {
                        layout: sprites.layout.clone(),
                        index: zone.sprite_index(),
                    }),
                    ..default()
                },
                Transform::from_xyz(
                    world_x,
                    world_y,
                    0.0,
                ),
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

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Pop: 0  Jobs: 0  Money: 0"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
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

    let window = if let Ok(w) = windows.single() {
        w
    } else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let (camera, cam_transform) = if let Ok(v) = camera_q.single() {
        v
    } else {
        return;
    };

    let Ok(world_pos) =
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
            // Update the sprite texture atlas index
            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.index = zone.sprite_index();
            }
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

    if let Ok(mut text) = query.single_mut() {
        **text = format!(
            "Pop: {}  Jobs: {}  Money: {}",
            stats.population, stats.jobs, stats.money
        );
    }
}
