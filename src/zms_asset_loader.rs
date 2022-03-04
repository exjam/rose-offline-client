use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset},
    prelude::Mesh,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use rose_file_readers::{RoseFile, ZmsFile};

use crate::render::{MESH_ATTRIBUTE_UV_1, MESH_ATTRIBUTE_UV_2, MESH_ATTRIBUTE_UV_3};

#[derive(Default)]
pub struct ZmsAssetLoader;

impl AssetLoader for ZmsAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            match <ZmsFile as RoseFile>::read(bytes.into(), &Default::default()) {
                Ok(mut zms) => {
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    mesh.set_indices(Some(Indices::U16(zms.indices)));

                    if !zms.position.is_empty() {
                        for vert in zms.position.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
                        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, zms.position);
                    }

                    if !zms.normal.is_empty() {
                        for vert in zms.normal.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
                        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, zms.normal);
                    }

                    if !zms.tangent.is_empty() {
                        for vert in zms.tangent.iter_mut() {
                            let y = vert[1];
                            vert[1] = vert[2];
                            vert[2] = -y;
                        }
                        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, zms.tangent);
                    }

                    if !zms.color.is_empty() {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, zms.color);
                    }

                    if !zms.bone_weights.is_empty() {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, zms.bone_weights);
                    }

                    if !zms.bone_indices.is_empty() {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, zms.bone_indices);
                    }

                    if !zms.uv1.is_empty() {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, zms.uv1);
                    }

                    if !zms.uv2.is_empty() {
                        mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, zms.uv2);
                    }

                    if !zms.uv3.is_empty() {
                        mesh.insert_attribute(MESH_ATTRIBUTE_UV_2, zms.uv3);
                    }

                    if !zms.uv4.is_empty() {
                        mesh.insert_attribute(MESH_ATTRIBUTE_UV_3, zms.uv4);
                    }

                    load_context.set_default_asset(LoadedAsset::new(mesh));
                    Ok(())
                }
                Err(error) => Err(error),
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["zms"]
    }
}
