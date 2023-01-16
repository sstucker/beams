use std::{f32::{EPSILON, consts::PI}, collections::btree_map::Iter};
use itertools_num::linspace;

use bevy::{prelude::*, window::PresentMode};
use bevy_prototype_lyon::prelude::*;

const WINDOW_W: usize = 1080;
const WINDOW_H: usize = 920;

const PX_PER_MM: usize = 20;

const RAY_DENSITY: f32 = 0.2;

#[inline]
pub fn cross2(a: Vec2, b: Vec2) -> f32 {
    return a[0]*b[1] - b[0]*a[1]
}

pub fn intersect(ray: &Ray, surface: &Surface) -> f32 {
    let v1 = ray.p - surface.p1;
    let v2 = surface.p2 - surface.p1;
    let v3 = Vec2::new(-ray.l[1], ray.l[0]);
    let dot = v2.dot(v3);
    if dot.abs() < 0.000001 {
        return f32::INFINITY
    } else {
        let cross = v2.perp_dot(v1);
        let t1 = cross / dot;
        let t2 = v1.dot(v3) / dot;
        if t1 >= 0.0 && (t2 >= 0.0 && t2 <= 1.0) {
            return t1
        } else {
            return f32::INFINITY
        }
    }
}

#[derive(Component, Clone)]
pub struct BeamSource {
    pub pos: Vec2,
    pub direction: Vec2,
    pub waist: f32,
    pub w: f32,
    pub index: f32
}

impl BeamSource {
    pub fn new(
        pos: Vec2,
        direction: Vec2,
        waist: f32
    ) -> Self {
        Self {
            pos: pos,
            direction: direction,
            waist: waist,
            w: 532.,
            index: 1.0
        }
    }
}

struct RaycastEvent {
    ray: Option<Ray>,
    tree: Option<Entity>
}

#[derive(Component, Clone)]
pub struct RayTree {
    root: Ray,
    branches: Vec::<Ray>
}

impl RayTree {
    pub fn new(ray: Ray) -> Self {
        Self {
            root: ray.clone(),
            branches: Vec::<Ray>::new()
        }
    }
}

#[derive(Component, Clone)]
pub struct RaySource;

#[derive(Component, Clone)]
pub struct RaySegment;

#[derive(Clone)]
pub struct Ray {
    pub p: Vec2,
    pub l: Vec2,
    pub i: f32,
    index: f32, 
    w: f32
}

impl Ray {
    pub fn new(p: Vec2, l: Vec2, index: f32) -> Self {
        Self {
            p: p, 
            l: l,
            i: 1.0, 
            index: index,
            w: 532.
        }
    }
}

#[derive(Component, Clone)]
pub struct Surface {
    pub p1: Vec2,
    pub p2: Vec2,
    pub dp: Vec2,
    pub normal: Vec2,
    pub length: f32,
    pub index: f32,
    pub reflection: f32,
    pub absorption: f32  
}

impl Surface {
    pub fn glass(
        p1: Vec2,
        p2: Vec2,
        index: f32,
    ) -> Self {
        Self {
            p1: p1,
            p2: p2,
            dp: p2 - p1,
            length: (p2 - p1).length(),
            normal: (p2 - p1).normalize().perp(),
            index: index,
            reflection: 0.0,
            absorption: 0.0
        }
    }
    pub fn blocker(
        p1: Vec2,
        p2: Vec2,
    ) -> Self {
        Self {
            p1: p1,
            p2: p2,
            dp: p2 - p1,
            length: (p2 - p1).length(),
            normal: (p2 - p1).normalize().perp(),
            index: 1.0,
            reflection: 0.0,
            absorption: 1.0
        }
    }
}

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Beams ".to_string() + env!("CARGO_PKG_VERSION"),
                width: WINDOW_W as f32,
                height: WINDOW_H as f32,
                present_mode: PresentMode::AutoNoVsync,
                // mode: WindowMode::BorderlessFullscreen,
                ..default()
            },
            ..default()
        }))
        .add_plugin(ShapePlugin)
        .add_event::<RaycastEvent>()
        .add_startup_system(draw_grid_system)
        .add_startup_system(setup_system)
        .add_system(draw_surface_system)
        .add_system(raycast_system)
        .run();
}

fn raycast_system(
    mut commands: Commands,
    reader: EventReader<RaycastEvent>,
    surface_query: Query<&Surface>
) {
    for raycast_event in reader.iter() {
        if let Some(ray) = raycast_event.ray {
            if let Some(old_tree) = raycast_event.tree {
                commands.entity(old_tree).despawn();
            }
            let tree = RayTree::new(ray);
            'surfaces: for surface in surface_query.iter() {
                let d = intersect(&ray, surface);
                if d.is_finite() && d > 0.1 {
                    println!("Intersection at {}", d);
                    let mut path_builder = PathBuilder::new();
                    path_builder.move_to(ray.p);
                    path_builder.line_to(ray.p + ray.l * d);
                    commands.spawn(GeometryBuilder::build_as(
                        &path_builder.build(),
                        DrawMode::Stroke(StrokeMode::new(Color::YELLOW, 1.0)),
                        Transform::default(),
                    )).insert(RaySegment);                
                    if surface.absorption < 1.0 {
                        let normal = if surface.normal.angle_between(ray.l) > surface.normal.angle_between(ray.l) {
                            surface.normal
                        } else {
                            -1. * surface.normal
                        };
                        let refracted = ((ray.index * normal.perp_dot(ray.l)) / surface.index).asin();
                        println!("incident is {} refracted is {}", ray.l.angle_between(normal), refracted);
                        tree.branches.push(Ray::new(
                            ray.p + ray.l * d,
                            Vec2::from_angle(refracted).normalize(),
                            surface.index
                        ));
                    }
                    break 'surfaces;
                }
            }
        }
    }
}

fn setup_system(
    mut commands: Commands,
    mut writer: EventWriter<RaycastEvent>
) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_translation(Vec3::new((WINDOW_W / 2) as f32, (WINDOW_H / 2) as f32, 0.)),
        ..Default::default()
    });

    let beam = BeamSource::new(
        Vec2::new(200., 650.),
        Vec2::new(1., -0.02).normalize(),
        10.
    )

    commands.spawn(beam);
    for x in linspace(-beam.waist / 2., beam.waist / 2., (beam.waist * RAY_DENSITY) as usize) {
        let beam_ray = Ray::new(
            beam.pos + x * Vec2::new(-beam.direction[1], beam.direction[0]),
            beam.direction,
            1.0
        )
        writer.send(RaycastEvent {
            ray: Some(beam_ray),
            tree: None
        })
    }


    commands.spawn(Surface::glass(
        Vec2::new(500., 600.), 
        Vec2::new(500., 700.),
        1.5
    ));
    commands.spawn(Surface::glass(
        Vec2::new(900., 600.), 
        Vec2::new(950., 700.),
        1.0
    ));
    commands.spawn(Surface::blocker(
        Vec2::new(0., 0.), 
        Vec2::new(WINDOW_W as f32, 0.),
    ));
    commands.spawn(Surface::blocker(
        Vec2::new(0., WINDOW_H as f32), 
        Vec2::new(WINDOW_W as f32, WINDOW_H as f32),
    ));
    commands.spawn(Surface::blocker(
        Vec2::new(0., 0.), 
        Vec2::new(0., WINDOW_H as f32),
    ));
    commands.spawn(Surface::blocker(
        Vec2::new(WINDOW_W as f32, 0.), 
        Vec2::new(WINDOW_W as f32, WINDOW_H as f32),
    ));
}

fn draw_surface_system(
    mut commands: Commands,
    query: Query<&Surface>
) {
    for surface in query.iter() {
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(surface.p1);
        path_builder.line_to(surface.p2);
        commands.spawn(GeometryBuilder::build_as(
            &path_builder.build(),
            DrawMode::Stroke(StrokeMode::new(Color::WHITE, 1.0)),
            Transform::default(),
        ));
    }
}

fn draw_grid_system(
    mut commands: Commands
) {
    for i in 0..(WINDOW_W / PX_PER_MM) {
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(Vec2::new((i * PX_PER_MM) as f32, 0.,));
        path_builder.line_to(Vec2::new((i * PX_PER_MM) as f32, WINDOW_H as f32,));
        commands.spawn(GeometryBuilder::build_as(
            &path_builder.build(),
            DrawMode::Stroke(StrokeMode::new(Color::rgb(0.5, 0.5, 0.5), 0.3)),
            Transform::default(),
        ));
    }
    for j in 0..(WINDOW_H / PX_PER_MM) {
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(Vec2::new(0., (j * PX_PER_MM) as f32,));
        path_builder.line_to(Vec2::new(WINDOW_W as f32, (j * PX_PER_MM) as f32,));
        commands.spawn(GeometryBuilder::build_as(
            &path_builder.build(),
            DrawMode::Stroke(StrokeMode::new(Color::rgb(0.5, 0.5, 0.5), 0.3)),
            Transform::default(),
        ));
    }
}