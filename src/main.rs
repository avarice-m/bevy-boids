#![allow(clippy::type_complexity)]
use std::{collections::HashMap, f32::consts::PI};

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
    .add_system(boid_sense.system().chain(boid_control.system()))
    .add_system(velocity.system())
    .add_system(angular_velocity.system())
    // mouse shenanigans
    .add_system(mouse_boid.system())
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

  mouse_setup(&mut commands, material_handle.clone(), mesh_handle.clone());

  commands.spawn_bundle(OrthographicCameraBundle::new_2d());
  for i in 0..5 {
    let i = i as f32;
    commands
      .spawn_bundle(SpriteBundle {
        material: material_handle.clone(),
        sprite: Sprite {
          size: Vec2::splat(10.0),
          ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(i * 10.0, i * 10.0, 0.0)),
        mesh: mesh_handle.clone(),
        ..Default::default()
      })
      .insert(Boid {
        neighbor_radius: 20.0,
        personal_radius: 2.0,
        coverage_angle: 1.5 * PI,
      })
      .insert(Velocity(100.0))
      .insert(AngularVelocity(0.5 * PI));
  }
}

// ===BOIDS===
struct Boid {
  neighbor_radius: f32,
  personal_radius: f32,
  coverage_angle: f32,
}

struct SensoryData
{
  neighbor_map: HashMap<Entity, Vec<Entity>>,
  too_close_map: HashMap<Entity, Vec<Entity>>,
}

// something that influences other boids but is not itself a boid
struct PseudoBoid;

fn boid_sense(boids: Query<(Entity, &Boid, &Transform)>, pseudo_boids: Query<Entity, With<(Transform, PseudoBoid)>>) -> SensoryData {
  // a map of vectors to track which boids can see which other boids
  let mut neighbor_map = HashMap::new();
  let mut too_close_map = HashMap::new();
  // outer loop picks one boid at a time to calculate sight
  for (entity, boid, Transform { translation, .. }) in boids.iter() {
    let mut neighbor_boids = Vec::new();
    let mut too_close_boids = Vec::new();

    // inner loop goes over all other boids and figures out which are seen
    for (
      other_entity,
      _,
      Transform {
        translation: other_translation,
        ..
      },
    ) in boids.iter()
    {
      // TODO add view angle, these boids are too powerful with 360° vision
      let distance = translation.distance(*other_translation);
      if distance <= boid.neighbor_radius {
        neighbor_boids.push(other_entity);
      }
      if distance <= boid.personal_radius {
        too_close_boids.push(other_entity);
      }

      neighbor_boids.extend(pseudo_boids.iter());
      too_close_boids.extend(pseudo_boids.iter());
    }

    neighbor_map.insert(entity, neighbor_boids);
    too_close_map.insert(entity, too_close_boids);
  }

  SensoryData
  {
    neighbor_map,
    too_close_map,
  }
}

// noop function to debug problems and determine which function its happening
// fn dump_chain(In(_): In<HashMap<Entity, Vec<Entity>>>) {}

fn boid_control(
  In(sensory_data): In<SensoryData>,
  boids: Query<Entity, (With<Transform>, With<Boid>)>,
  transforms: Query<&Transform, With<Boid>>,
  mut ang_velocities: Query<&mut AngularVelocity, With<Boid>>,
) {
  for entity in boids.iter() {
    let neighbors = sensory_data.neighbor_map.get(&entity).unwrap()
      .iter().map(|a| transforms.get(*a).unwrap()).collect();
    let too_close = sensory_data.too_close_map.get(&entity).unwrap()
      .iter().map(|a| transforms.get(*a).unwrap()).collect();

    let this = transforms.get(entity).unwrap();
    let cohesion = cohesion(this, neighbors);
    let separation = separation(this, too_close);

    let forward = this.rotation.mul_vec3(Vec3::Y).normalize();
    let cohesion_angle = radians_to(forward, cohesion);
    let separation_angle = radians_to(forward, separation);
    let radians_delta = (cohesion_angle + separation_angle) / 2.0;

    let mut ang_vel = ang_velocities.get_mut(entity).unwrap();
    ang_vel.0 = radians_delta;
  }
}

fn radians_to(forward: Vec3, desired: Vec3) -> f32
{
  if desired == Vec3::ZERO
  {
    0.0
  }
  else
  {
    let delta = forward.angle_between(desired);
    let cross = forward.cross(desired);

    // delta is always positive, so we need to determine the direction the
    // delta should be in.
    if cross.z > 0.0
    {
      delta
    }
    else
    {
      -delta
    }
  }
}

fn cohesion(this: &Transform, boids: Vec<&Transform>) -> Vec3
{
  let neighbors: Vec<Vec3> = boids.iter()
    .map(|b| b.translation)
    .collect();

  let sum = neighbors.iter().sum::<Vec3>();
  let count = Vec3::splat(neighbors.len() as f32);
  let dir = ((sum / count) - this.translation).normalize();

  if dir.is_nan()
  {
    Vec3::ZERO
  }
  else
  {
    dir
  }
}

fn separation(this: &Transform, boids: Vec<&Transform>) -> Vec3
{
  let neighbors: Vec<Vec3> = boids.iter()
    .map(|b| b.translation)
    .collect();

  let sum = neighbors.iter().sum::<Vec3>();
  let count = Vec3::splat(neighbors.len() as f32);
  let dir = (this.translation - (sum / count)).normalize();

  if dir.is_nan()
  {
    Vec3::ZERO
  }
  else
  {
    dir
  }
}

// ===PHYSICS===

// units per second
struct Velocity(f32);

fn velocity(mut query: Query<(&Velocity, &mut Transform)>, time: Res<Time>) {
  for (velocity, mut transform) in query.iter_mut() {
    let direction = transform.rotation.mul_vec3(Vec3::Y);
    transform.translation += time.delta_seconds() * velocity.0 * direction;
  }
}

// radians per second, positive is clockwise negative is counter clockwise
struct AngularVelocity(f32);

fn angular_velocity(mut query: Query<(&AngularVelocity, &mut Transform)>, time: Res<Time>) {
  for (ang_vel, mut transform) in query.iter_mut() {
    transform.rotate(Quat::from_rotation_z(ang_vel.0 * time.delta_seconds()))
  }
}

// ===MOUSE SHENANIGANS===
struct MouseBoid;

fn mouse_setup(commands: &mut Commands, material: Handle<ColorMaterial>, mesh: Handle<Mesh>) {
  for _ in 0..5 {
    commands
      .spawn()
      .insert(PseudoBoid)
      .insert(MouseBoid)
      .insert_bundle(SpriteBundle {
        transform: Transform::from_translation(Vec3::ZERO),
        sprite: Sprite::new(Vec2::splat(5.0)),
        material: material.clone(),
        mesh: mesh.clone(),
        ..Default::default()
      });
  }
}

fn mouse_boid(
  mut mouse: EventReader<CursorMoved>,
  mut query: Query<&mut Transform, With<MouseBoid>>,
  windows: Res<Windows>,
) {
  let window = windows.get_primary().unwrap();
  let size = Vec2::new(window.width(), window.height());
  for ev in mouse.iter() {
    let new_position = ev.position - size / 2.0;
    for mut transform in query.iter_mut() {
      transform.translation = new_position.extend(0.0);
    }
  }
}
