use std::f32::consts::PI;

use anyhow::Result;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::pbr::AmbientLight;
use bevy::prelude::shape;
use bevy::prelude::*;
use bevy::render::texture::ImagePlugin;
use bevy::window::WindowPlugin;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use radarc::coverage::RadarCoverageCalculator;
use radarc::dem::{DigitalElevationModel, RadarSite};

const XY_SCALE: f32 = 0.001; // meters -> kilometers
const Z_SCALE: f32 = 0.001; // meters -> kilometers
const DEFAULT_POINT_SCALE: f32 = 0.35;

fn main() {
    if let Err(err) = launch_viewer() {
        eprintln!("Failed to start viewer: {err:?}");
    }
}

fn launch_viewer() -> Result<()> {
    let dem = DigitalElevationModel::from_json_file("data/sample_dem.json")?;
    let radar_site = RadarSite {
        x_m: 0.0,
        y_m: 0.0,
        height_agl_m: 15.0,
    };

    let calculator =
        RadarCoverageCalculator::new(&dem, radar_site, Some(25.0), 20.0, 150.0, 4.0 / 3.0, None)?;
    let coverage = calculator.compute();
    let coverage_data = CoverageData::from_dem_and_result(&dem, coverage);

    App::new()
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
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (radar_ui_panel, update_point_visuals, orbit_camera_input),
        )
        .run();

    Ok(())
}

#[derive(Resource, Clone)]
struct CoverageData {
    result: radarc::coverage::CoverageResult,
    center_world_m: Vec2,
    plane_size_km: Vec2,
    radar_translation: Vec3,
}

impl CoverageData {
    fn from_dem_and_result(
        dem: &DigitalElevationModel,
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

    let radar_mesh = meshes.add(Mesh::from(shape::Cylinder {
        radius: 0.18,
        height: 1.0,
        ..Default::default()
    }));
    let radar_material = materials.add(Color::rgb(0.95, 0.95, 0.2).into());
    commands.spawn(PbrBundle {
        mesh: radar_mesh.clone(),
        material: radar_material,
        transform: Transform::from_translation(data.radar_translation)
            .with_scale(Vec3::new(1.0, 2.0, 1.0)),
        ..Default::default()
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
        settings.point_scale,
        &data,
    );

    spawn_points(
        &mut commands,
        &data.result.occluded,
        CoverageCategory::Occluded,
        sphere_mesh,
        occluded_material,
        settings.point_scale,
        &data,
    );
}

fn spawn_points(
    commands: &mut Commands,
    points: &[radarc::coverage::CoveragePoint],
    category: CoverageCategory,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    point_scale: f32,
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
        let mut transform =
            Transform::from_translation(Vec3::new(anchor.xy.x, anchor.height, anchor.xy.y));
        transform.scale = Vec3::splat(point_scale);
        commands
            .spawn(PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform,
                ..Default::default()
            })
            .insert(category)
            .insert(anchor)
            .insert(Visibility::Visible);
    }
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
