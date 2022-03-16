use bevy::prelude::{AssetServer, Assets, Commands, Entity};

use rose_data::NpcId;
use rose_file_readers::{ChrFile, VfsIndex, ZmdFile, ZscFile};

use crate::{
    character_model::{spawn_model, spawn_skeleton},
    components::{ModelSkeleton, NpcModel},
    render::StaticMeshMaterial,
};

pub struct NpcModelList {
    chr_npc: ChrFile,
    zsc_npc: ZscFile,
}

impl NpcModelList {
    pub fn new(vfs: &VfsIndex) -> Result<NpcModelList, anyhow::Error> {
        Ok(NpcModelList {
            chr_npc: vfs.read_file::<ChrFile, _>("3DDATA/NPC/LIST_NPC.CHR")?,
            zsc_npc: vfs.read_file::<ZscFile, _>("3DDATA/NPC/PART_NPC.ZSC")?,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_npc_model(
    commands: &mut Commands,
    model_entity: Entity,
    npc_model_list: &NpcModelList,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    npc_id: NpcId,
    vfs: &VfsIndex,
) -> Option<(NpcModel, ModelSkeleton)> {
    let npc_model_data = npc_model_list.chr_npc.npcs.get(&npc_id.get())?;
    let model_skeleton = if let Some(skeleton) = npc_model_list
        .chr_npc
        .skeleton_files
        .get(npc_model_data.skeleton_index as usize)
        .and_then(|p| vfs.read_file::<ZmdFile, _>(p).ok())
    {
        spawn_skeleton(commands, model_entity, &skeleton)
    } else {
        ModelSkeleton {
            bones: Vec::new(),
            dummy_bone_offset: 0,
        }
    };

    let mut model_parts = Vec::with_capacity(16);
    for model_id in npc_model_data.model_ids.iter() {
        let (_model_id, mut parts) = spawn_model(
            commands,
            model_entity,
            asset_server,
            static_mesh_materials,
            &npc_model_list.zsc_npc,
            *model_id as usize,
            &model_skeleton,
            None,
        );
        model_parts.append(&mut parts);
    }

    Some((
        NpcModel {
            npc_id,
            model_parts,
        },
        model_skeleton,
    ))
}
