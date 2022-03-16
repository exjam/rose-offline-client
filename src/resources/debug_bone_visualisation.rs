use bevy::{
    pbr::{AlphaMode, StandardMaterial},
    prelude::{shape, Assets, Color, FromWorld, Handle, Mesh},
};

pub struct DebugBoneVisualisation {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
}

impl FromWorld for DebugBoneVisualisation {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh = meshes.add(Mesh::from(shape::Cube { size: 0.1 }));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let material = materials.add(StandardMaterial {
            base_color: Color::rgba(1.0, 0.08, 0.58, 0.75),
            alpha_mode: AlphaMode::Blend,
            ..Default::default()
        });

        Self { mesh, material }
    }
}
