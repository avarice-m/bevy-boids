use std::f32::consts::PI;

use bevy::{
  prelude::*,
  render::{mesh::Indices, pipeline::PrimitiveTopology},
};

const INDICES: [u16; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];
const POSITIONS: [f32; 6] = [0.0, 0.0, 5.0, -10.0, -5.0, -10.0];

fn main() {
  App::build()
    .insert_resource(WindowDescriptor {
      height: 500.0,
      width: 500.0,
      ..Default::default()
    })
    .insert_resource(ClearColor(Color::rgb(0.3, 0.2, 0.25)))
    .add_plugins(DefaultPlugins)
    .add_startup_system(setup.system())
    .add_system(boid_move.system())
    .run();
}

fn setup(
  mut commands: Commands,
  mut materials: ResMut<Assets<ColorMaterial>>,
  mut meshes: ResMut<Assets<Mesh>>,
) {
  let material_handle = materials.add(Color::OLIVE.into());
  let mesh_handle = {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U16(INDICES.to_vec())));
    mesh.set_attribute(
      Mesh::ATTRIBUTE_POSITION,
      vec![[0.0, 0.0], [1.0, -3.0], [-1.0, -3.0]],
    );
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0; 3]; 3]);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0; 2]; 3]);
    meshes.add(mesh)
  };

  commands.spawn_bundle(OrthographicCameraBundle::new_2d());
  commands
    .spawn_bundle(SpriteBundle {
      material: material_handle.clone(),
      sprite: Sprite {
        size: Vec2::splat(10.0),
        ..Default::default()
      },
      mesh: mesh_handle,
      ..Default::default()
    })
    .insert(Boid {
      radius: 20.0,
      coverage_angle: 1.5 * PI,
    });
}

fn boid_move(mut boids: Query<(&Boid, &mut Transform)>) {
  for (_boid, mut transform) in boids.iter_mut() {
    transform.translation.y += 10.0;
  }
}

struct Boid {
  radius: f32,
  coverage_angle: f32,
}

// Something that boids will follow like a fellow boid, but
// has custom movement
struct PseudoBoid;
