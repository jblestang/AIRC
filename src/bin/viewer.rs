use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::pbr::AmbientLight;
use bevy::prelude::shape;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::texture::ImagePlugin;
use bevy::render::view::screenshot::ScreenshotManager;
use bevy::window::{PrimaryWindow, WindowPlugin};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use radarc::coverage::RadarCoverageCalculator;
use radarc::dem::{DigitalElevationModel, RadarSite};

const XY_SCALE: f32 = 0.001; // meters -> kilometers
const Z_SCALE: f32 = 0.001; // meters -> kilometers
const DEFAULT_POINT_SCALE: f32 = 0.35;

fn main() {
    let capture_wireframe = std::env::args().any(|arg| arg == "--capture-wireframe");
    if let Err(err) = launch_viewer(ViewerOptions { capture_wireframe }) {
        eprintln!("Failed to start viewer: {err:?}");
    }
}

fn launch_viewer(options: ViewerOptions) -> Result<()> {
    let dem = DigitalElevationModel::from_json_file("data/sample_dem.json")?;
    let radar_site = RadarSite {
        x_m: 0.0,
        y_m: 0.0,
        height_agl_m: 15.0,
    };

    let coverage = {
        let calculator = RadarCoverageCalculator::new(
            &dem,
            radar_site,
            Some(25.0),
            20.0,
            150.0,
            4.0 / 3.0,
            None,
        )?;
        calculator.compute()
    };
    let coverage_data = CoverageData::from_dem_and_result(dem, coverage);

    App::new()
        .insert_resource(options)
        .insert_resource(ScreenshotState::default())
        .insert_resource(coverage_data)
        .insert_resource(VisualizationSettings::default())
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "RadarC 3D Viewer".into(),
                        resolution: (1200.0, 800.0).into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((
            EguiPlugin,
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
            WireframePlugin,
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                radar_ui_panel,
                update_point_visuals,
                update_radar_marker,
                update_terrain_surface,
                orbit_camera_input,
            ),
        )
        .run();

    Ok(())
}

#[derive(Resource, Clone)]
struct CoverageData {
    result: radarc::coverage::CoverageResult,
    dem: DigitalElevationModel,
    center_world_m: Vec2,
    plane_size_km: Vec2,
    radar_translation: Vec3,
}

impl CoverageData {
    fn from_dem_and_result(
        dem: DigitalElevationModel,
        result: radarc::coverage::CoverageResult,
    ) -> Self {
        let (min_x, min_y, max_x, max_y) = dem.extent();
        let center_world_m = Vec2::new(
            ((min_x + max_x) / 2.0) as f32,
            ((min_y + max_y) / 2.0) as f32,
        );
        let plane_size_km = Vec2::new(
            ((max_x - min_x) as f32) * XY_SCALE,
            ((max_y - min_y) as f32) * XY_SCALE,
        );
        let radar_translation = Vec3::new(
            ((result.radar_site.x_m as f32) - center_world_m.x) * XY_SCALE,
            (result.radar_altitude_m as f32) * Z_SCALE,
            ((result.radar_site.y_m as f32) - center_world_m.y) * XY_SCALE,
        );
        Self {
            result,
            dem,
            center_world_m,
            plane_size_km,
            radar_translation,
        }
    }
}

#[derive(Resource)]
struct VisualizationSettings {
    show_visible: bool,
    show_occluded: bool,
    point_scale: f32,
    height_exaggeration: f32,
}

#[derive(Resource, Clone, Copy)]
struct ViewerOptions {
    capture_wireframe: bool,
}

#[derive(Resource, Default)]
struct ScreenshotState {
    requested: bool,
    saved: bool,
}

impl Default for VisualizationSettings {
    fn default() -> Self {
        Self {
            show_visible: true,
            show_occluded: true,
            point_scale: DEFAULT_POINT_SCALE,
            height_exaggeration: 1.2,
        }
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum CoverageCategory {
    Visible,
    Occluded,
}

#[derive(Component)]
struct PointAnchor {
    xy: Vec2,
    height: f32,
}

#[derive(Component)]
struct TerrainSurface;

#[derive(Component)]
struct RadarMarker {
    base_height: f32,
}

#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    radius: f32,
    azimuth: f32,
    elevation: f32,
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    data: Res<CoverageData>,
    settings: Res<VisualizationSettings>,
) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: false,
            ..Default::default()
        },
        transform: Transform::from_xyz(8.0, 12.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    let mut camera_transform = Transform::from_xyz(15.0, 12.0, 15.0);
    camera_transform.look_at(Vec3::ZERO, Vec3::Y);
    commands
        .spawn(Camera3dBundle {
            transform: camera_transform,
            ..Default::default()
        })
        .insert(OrbitCamera {
            focus: Vec3::ZERO,
            radius: 24.0,
            azimuth: 0.8,
            elevation: 0.6,
        });

    let plane_mesh = meshes.add(Mesh::from(shape::Plane::from_size(1.0)));
    let plane_material = materials.add(Color::rgb(0.15, 0.18, 0.2).into());
    commands.spawn(PbrBundle {
        mesh: plane_mesh,
        material: plane_material,
        transform: Transform::from_scale(Vec3::new(
            data.plane_size_km.x.max(0.1),
            1.0,
            data.plane_size_km.y.max(0.1),
        )),
        ..Default::default()
    });

    let terrain_mesh = meshes.add(build_dem_mesh(&data.dem, data.center_world_m));
    let terrain_material = materials.add(StandardMaterial {
        base_color: Color::rgba(0.2, 0.4, 0.6, 0.05),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..Default::default()
    });
    commands
        .spawn(PbrBundle {
            mesh: terrain_mesh,
            material: terrain_material,
            transform: Transform::from_scale(Vec3::new(1.0, settings.height_exaggeration, 1.0)),
            ..Default::default()
        })
        .insert(TerrainSurface)
        .insert(Wireframe);

    let radar_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 0.18,
        height: 1.0,
        ..Default::default()
    }));
    let radar_material = materials.add(Color::rgb(0.95, 0.95, 0.2).into());
    commands
        .spawn(PbrBundle {
            mesh: radar_mesh.clone(),
            material: radar_material,
            transform: Transform::from_translation(Vec3::new(
                data.radar_translation.x,
                data.radar_translation.y * settings.height_exaggeration,
                data.radar_translation.z,
            ))
            .with_scale(Vec3::new(1.0, 2.0, 1.0)),
            ..Default::default()
        })
        .insert(RadarMarker {
            base_height: data.radar_translation.y,
        });

    let visible_material = materials.add(Color::rgb(0.15, 0.82, 0.45).into());
    let occluded_material = materials.add(Color::rgb(0.9, 0.28, 0.28).into());
    let sphere_mesh = meshes.add(Mesh::from(shape::Icosphere {
        radius: 0.5,
        subdivisions: 3,
    }));

    spawn_points(
        &mut commands,
        &data.result.visible,
        CoverageCategory::Visible,
        sphere_mesh.clone(),
        visible_material.clone(),
        &settings,
        &data,
    );

    spawn_points(
        &mut commands,
        &data.result.occluded,
        CoverageCategory::Occluded,
        sphere_mesh,
        occluded_material,
        &settings,
        &data,
    );
}

fn spawn_points(
    commands: &mut Commands,
    points: &[radarc::coverage::CoveragePoint],
    category: CoverageCategory,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    settings: &VisualizationSettings,
    data: &CoverageData,
) {
    for point in points {
        let anchor = PointAnchor {
            xy: Vec2::new(
                ((point.x_m as f32) - data.center_world_m.x) * XY_SCALE,
                ((point.y_m as f32) - data.center_world_m.y) * XY_SCALE,
            ),
            height: (point.target_altitude_m as f32) * Z_SCALE,
        };
        let mut transform = Transform::from_translation(Vec3::new(
            anchor.xy.x,
            anchor.height * settings.height_exaggeration,
            anchor.xy.y,
        ));
        transform.scale = Vec3::splat(settings.point_scale);
        let show = match category {
            CoverageCategory::Visible => settings.show_visible,
            CoverageCategory::Occluded => settings.show_occluded,
        };
        commands
            .spawn(PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform,
                ..Default::default()
            })
            .insert(category)
            .insert(anchor)
            .insert(if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
    }
}

fn build_dem_mesh(dem: &DigitalElevationModel, center_world_m: Vec2) -> Mesh {
    let width = dem.width();
    let height = dem.height();
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(width * height);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(width * height);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(width * height);

    let max_u = (width.saturating_sub(1)).max(1) as f32;
    let max_v = (height.saturating_sub(1)).max(1) as f32;

    for row in 0..height {
        for col in 0..width {
            let x_world = dem.origin_x_m + col as f64 * dem.cell_size_m;
            let y_world = dem.origin_y_m + row as f64 * dem.cell_size_m;
            let elevation = dem.elevation_value(row, col);
            positions.push([
                ((x_world as f32) - center_world_m.x) * XY_SCALE,
                (elevation as f32) * Z_SCALE,
                ((y_world as f32) - center_world_m.y) * XY_SCALE,
            ]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([col as f32 / max_u, row as f32 / max_v]);
        }
    }

    let mut indices: Vec<u32> = Vec::new();
    if width > 1 && height > 1 {
        for row in 0..(height - 1) {
            for col in 0..(width - 1) {
                let i0 = (row * width + col) as u32;
                let i1 = (row * width + col + 1) as u32;
                let i2 = ((row + 1) * width + col) as u32;
                let i3 = ((row + 1) * width + col + 1) as u32;
                indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if !indices.is_empty() {
        mesh.set_indices(Some(Indices::U32(indices)));
    }
    mesh
}

fn radar_ui_panel(
    mut contexts: EguiContexts,
    mut settings: ResMut<VisualizationSettings>,
    data: Res<CoverageData>,
) {
    egui::Window::new("Radar controls").show(contexts.ctx_mut(), |ui| {
        ui.label(format!(
            "Radar altitude: {:.1} m",
            data.result.radar_altitude_m
        ));
        ui.label(format!("Visible cells: {}", data.result.visible.len()));
        ui.label(format!("Occluded cells: {}", data.result.occluded.len()));
        ui.separator();
        ui.checkbox(&mut settings.show_visible, "Show visible");
        ui.checkbox(&mut settings.show_occluded, "Show occluded");
        ui.add(egui::Slider::new(&mut settings.point_scale, 0.1..=1.5).text("Marker scale"));
        ui.add(
            egui::Slider::new(&mut settings.height_exaggeration, 0.5..=5.0)
                .text("Height exaggeration"),
        );
        ui.label("Scroll wheel: zoom | Right mouse drag: orbit camera");
    });
}

fn update_point_visuals(
    settings: Res<VisualizationSettings>,
    mut query: Query<(
        &CoverageCategory,
        &PointAnchor,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    if !settings.is_changed() {
        return;
    }
    for (category, anchor, mut transform, mut visibility) in &mut query {
        transform.translation = Vec3::new(
            anchor.xy.x,
            anchor.height * settings.height_exaggeration,
            anchor.xy.y,
        );
        transform.scale = Vec3::splat(settings.point_scale);
        let show = match category {
            CoverageCategory::Visible => settings.show_visible,
            CoverageCategory::Occluded => settings.show_occluded,
        };
        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_radar_marker(
    settings: Res<VisualizationSettings>,
    mut query: Query<(&RadarMarker, &mut Transform)>,
) {
    if !settings.is_changed() {
        return;
    }
    for (marker, mut transform) in &mut query {
        transform.translation.y = marker.base_height * settings.height_exaggeration;
    }
}

fn update_terrain_surface(
    settings: Res<VisualizationSettings>,
    mut query: Query<&mut Transform, With<TerrainSurface>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut transform in &mut query {
        transform.scale.y = settings.height_exaggeration;
    }
}

const OUTPUT_SCREENSHOT: &str = "artifacts/viewer_wireframe.png";

fn capture_wireframe_screenshot(
    options: Res<ViewerOptions>,
    mut state: ResMut<ScreenshotState>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    window_query: Query<Entity, With<PrimaryWindow>>,
) {
    if !options.capture_wireframe || state.saved {
        return;
    }
    let Ok(window) = window_query.get_single() else {
        return;
    };
    let path = PathBuf::from(OUTPUT_SCREENSHOT);
    if !state.requested {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if screenshot_manager
            .save_screenshot(window, path.clone())
            .is_ok()
        {
            state.requested = true;
        }
    } else if screenshot_manager.active_screenshot_count() == 0 {
        state.saved = true;
        println!("Saved wireframe screenshot to {}", path.display());
    }
}

fn orbit_camera_input(
    mut motion_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    buttons: Res<Input<MouseButton>>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    let mut delta = Vec2::ZERO;
    if buttons.pressed(MouseButton::Right) {
        for ev in motion_events.read() {
            delta += ev.delta;
        }
    } else {
        motion_events.clear();
    }

    let mut scroll_total = 0.0;
    for ev in scroll_events.read() {
        scroll_total += ev.y;
    }

    for (mut orbit, mut transform) in &mut query {
        if delta.length_squared() > 0.0 {
            orbit.azimuth -= delta.x * 0.005;
            orbit.elevation = (orbit.elevation + delta.y * 0.005).clamp(0.05, PI - 0.05);
        }
        if scroll_total.abs() > f32::EPSILON {
            orbit.radius = (orbit.radius - scroll_total * 0.5).clamp(3.0, 120.0);
        }

        let sin_e = orbit.elevation.sin();
        let cos_e = orbit.elevation.cos();
        let focus = orbit.focus;
        let position = Vec3::new(
            focus.x + orbit.radius * cos_e * orbit.azimuth.cos(),
            focus.y + orbit.radius * sin_e,
            focus.z + orbit.radius * cos_e * orbit.azimuth.sin(),
        );
        *transform = Transform::from_translation(position).looking_at(focus, Vec3::Y);
    }
}
