use std::{f32::EPSILON, collections::btree_map::Iter};

use bevy::{prelude::*, window::PresentMode};
use bevy_prototype_lyon::prelude::*;

const WINDOW_W: usize = 1080;
const WINDOW_H: usize = 920;

const PX_PER_MM: usize = 20;

pub fn intersect(ray: &Ray, surface: &Surface) -> f32 {
    let v1 = ray.p - surface.p1;
    let v2 = surface.p2 - surface.p1;
    let v3 = Vec2::new(-ray.l[1], ray.l[0]);
    let dot = v2.dot(v3);
    if dot.abs() < 0.000001 {
        return f32::INFINITY
    } else {
        let cross = v2[0] * v1[1] - v2[1] * v1[0];
        let t1 = cross / dot;
        let t2 =cross / dot;
        if t1 >= 0.0 && t2 >= 0.0 && t2 <= 1.0 {
            return t1
        } else {
            return f32::INFINITY
        }
    }
}

#[derive(Component, Clone)]
pub struct Ray {
    pub l: Vec2,
    pub p: Vec2,
    n: f32, 
    w: f32
}

impl Ray {
    fn new(l: Vec2, p: Vec2) -> Self {
        Self {
            l: l, 
            p: p, 
            n: 1.0,
            w: 532.
        }
    }
}

#[derive(Component, Clone)]
pub struct Surface {
    pub p1: Vec2,
    pub p2: Vec2,
    pub dp: Vec2,
    pub length: f32,
    pub n: f32  
}

impl Surface {
    pub fn new(
        p1: Vec2,
        p2: Vec2,
        n: f32
    ) -> Self {
        Self {
            p1: p1,
            p2: p2,
            dp: p2 - p1,
            length: (p2 - p1).length(),
            n: n
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
        .add_startup_system(draw_grid_system)
        .add_startup_system(setup_system)
        .add_system(draw_surface_system)
        .add_system(raycast_system)
        .run();
}

fn raycast_system(
    mut commands: Commands,
    ray_query: Query<&Ray>,
    surface_query: Query<&Surface>
) {
    for ray in ray_query.iter() {
        for surface in surface_query.iter() {
            let d = intersect(ray, surface);
            if d.is_finite() {
                println!("Intersection at {}", d);
            } else {
                println!("No intersection.");
            }
        }
    }
}

fn setup_system(
    mut commands: Commands
) {
    commands.spawn(Camera2dBundle {
        transform: Transform::from_translation(Vec3::new((WINDOW_W / 2) as f32, (WINDOW_H / 2) as f32, 0.)),
        ..Default::default()
    });
    commands.spawn(Ray::new(
        Vec2::new(200., 650.),
        Vec2::new(1., 0.),
    ));
    commands.spawn(Ray::new(
        Vec2::new(200., 650.),
        Vec2::new(-1., 0.),
    ));
    commands.spawn(Surface::new(
        Vec2::new(500., 600.), 
        Vec2::new(500., 700.),
        1.5
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