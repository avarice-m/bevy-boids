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
        radius: 20.0,
        coverage_angle: 1.5 * PI,
      })
      .insert(Velocity(100.0))
      .insert(AngularVelocity(0.5 * PI));
  }
}

// ===BOIDS===
struct Boid {
  radius: f32,
  coverage_angle: f32,
}

// something that influences other boids but is not itself a boid
struct PseudoBoid;

fn boid_sense(boids: Query<(Entity, &Boid, &Transform)>) -> HashMap<Entity, Vec<Entity>> {
  // a map of vectors to track which boids can see which other boids
  let mut map = HashMap::new();
  // outer loop picks one boid at a time to calculate sight
  for (entity, boid, Transform { translation, .. }) in boids.iter() {
    let other_boids = {
      if let Some(others) = map.get_mut(&entity) {
        others
      } else {
        map.insert(entity, Vec::new());
        map.get_mut(&entity).unwrap()
      }
    };
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
      if translation.distance(*other_translation) <= boid.radius {
        other_boids.push(other_entity);
      }
    }
  }
  map
}

// noop function to debug problems and determine which function its happening
// fn dump_chain(In(_): In<HashMap<Entity, Vec<Entity>>>) {}

fn boid_control(
  In(sight_map): In<HashMap<Entity, Vec<Entity>>>,
  boids: Query<Entity, (With<Transform>, With<Boid>)>,
  transforms: Query<&Transform, With<Boid>>,
  pseudo_boids: Query<&Transform, With<PseudoBoid>>,
  mut ang_velocities: Query<&mut AngularVelocity, With<Boid>>,
) {
  for entity in boids.iter() {
    let others = sight_map.get(&entity).unwrap();
    if !others.is_empty() {
      let translations: Vec<Vec3> = others
        .iter()
        .map(|other| transforms.get(*other).unwrap().translation)
        .chain(pseudo_boids.iter().map(|pb| pb.translation))
        .collect();

      let cohesion_target = translations.iter().sum::<Vec3>()
                          / Vec3::splat(translations.len() as f32);

      let transform = transforms.get(entity).unwrap();
      let target_vector = cohesion_target - transform.translation;
      let radians_delta = {
        let delta =
          transform.rotation.mul_vec3(Vec3::Y).normalize()
          .angle_between(target_vector.normalize());

        let cross =
          transform.rotation.mul_vec3(Vec3::Y).normalize()
          .cross(target_vector.normalize());

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
      };
      let mut ang_vel = ang_velocities.get_mut(entity).unwrap();

      if radians_delta.is_nan()
      {
        // if the delta is NaN, then the cohesion target is the same as the
        // current position of the boid. In this case, we would like to let the
        // boid continue travelling in the same direction.
        ang_vel.0 = 0.0;
      }
      else
      {
        ang_vel.0 = radians_delta;
      }
    }
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
