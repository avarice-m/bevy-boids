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
      mesh: mesh_handle.clone(),
      ..Default::default()
    })
    .insert(Boid {
      radius: 20.0,
      coverage_angle: 1.5 * PI,
    })
    .insert(Velocity(0.0));
  commands
    .spawn_bundle(SpriteBundle {
      material: material_handle.clone(),
      sprite: Sprite {
        size: Vec2::splat(10.0),
        ..Default::default()
      },
      transform: Transform::from_translation(Vec3::new(5.0, 5.0, 0.0)),
      mesh: mesh_handle.clone(),
      ..Default::default()
    })
    .insert(Boid {
      radius: 20.0,
      coverage_angle: 1.5 * PI,
    })
    .insert(Velocity(0.0));
  commands
    .spawn_bundle(SpriteBundle {
      material: material_handle.clone(),
      sprite: Sprite {
        size: Vec2::splat(10.0),
        ..Default::default()
      },
      transform: Transform::from_translation(Vec3::new(-5.0, 5.0, 0.0)),
      mesh: mesh_handle.clone(),
      ..Default::default()
    })
    .insert(Boid {
      radius: 20.0,
      coverage_angle: 1.5 * PI,
    })
    .insert(Velocity(0.0));
}

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
      if translation.distance(*other_translation) <= boid.radius {
        other_boids.push(other_entity);
      }
    }
  }
  map
}

fn dump_chain(In(_): In<HashMap<Entity, Vec<Entity>>>) {}

fn boid_control(
  In(sight_map): In<HashMap<Entity, Vec<Entity>>>,
  boids: Query<Entity, (With<Transform>, With<Boid>)>,
  mut transforms: QuerySet<(
    Query<&Transform, With<Boid>>,
    Query<&mut Transform, With<Boid>>,
  )>,
) {
  for entity in boids.iter() {
    let others = sight_map.get(&entity).unwrap();
    if !others.is_empty() {
      let cohesion_target = others
        .iter()
        .map(|other| transforms.q0().get(*other).unwrap().translation)
        .reduce(|mut total, t| {
          total += t;
          total
        })
        .unwrap()
        / Vec3::splat(transforms.q0().iter().count() as f32);

      let rotation = {
        let transform = transforms.q1_mut().get_mut(entity).unwrap();
        let target_facing = cohesion_target - transform.translation;
        Quat::from_rotation_arc(
          transform.rotation.mul_vec3(Vec3::Y),
          target_facing.normalize(),
        )
      };

      transforms
        .q1_mut()
        .get_mut(entity)
        .unwrap()
        .rotate(rotation);
    }
  }
}

struct Boid {
  radius: f32,
  coverage_angle: f32,
}

// Something that boids will follow like a fellow boid, but
// has custom movement
struct PseudoBoid;

fn velocity(mut query: Query<(&Velocity, &mut Transform)>, time: Res<Time>) {
  for (velocity, mut transform) in query.iter_mut() {
    let direction = transform.rotation.mul_vec3(Vec3::Y);
    transform.translation += time.delta_seconds() * velocity.0 * direction;
  }
}

// units per second
struct Velocity(f32);
