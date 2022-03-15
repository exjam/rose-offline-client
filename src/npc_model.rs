use bevy::{
    pbr::StandardMaterial,
    prelude::{AssetServer, Assets, Commands, GlobalTransform, Handle, Mesh, Transform},
};

use rose_data::NpcId;
use rose_file_readers::{ChrFile, VfsIndex, ZmdFile, ZscFile};

use crate::{
    character_model::{spawn_model, spawn_skeleton},
    components::{CharacterModelSkeleton, NpcModel},
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

pub fn spawn_npc_model(
    commands: &mut Commands,
    npc_model_list: &NpcModelList,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    npc_id: NpcId,
    vfs: &VfsIndex,
    bone_visualisation: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
) -> Option<NpcModel> {
    let npc_model_data = npc_model_list.chr_npc.npcs.get(&npc_id.get())?;
    let skeleton = if let Some(skeleton) = npc_model_list
        .chr_npc
        .skeleton_files
        .get(npc_model_data.skeleton_index as usize)
        .and_then(|p| vfs.read_file::<ZmdFile, _>(p).ok())
    {
        spawn_skeleton(commands, &skeleton, bone_visualisation)
    } else {
        CharacterModelSkeleton {
            root: commands
                .spawn_bundle((Transform::default(), GlobalTransform::default()))
                .id(),
            bones: Vec::new(),
            dummy_bone_offset: 0,
        }
    };

    for model_id in npc_model_data.model_ids.iter() {
        spawn_model(
            commands,
            asset_server,
            static_mesh_materials,
            &npc_model_list.zsc_npc,
            *model_id as usize,
            &skeleton,
            None,
        );
    }

    Some(NpcModel { npc_id, skeleton })
}
