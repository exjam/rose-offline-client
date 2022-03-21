use bevy::{
    math::{Mat4, Quat, Vec3},
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, ComputedVisibility, Entity, GlobalTransform,
        Mesh, Transform, Visibility,
    },
    render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
};
use enum_map::EnumMap;

use rose_data::{EquipmentIndex, ItemType};
use rose_file_readers::{VfsIndex, ZmdFile, ZscFile};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment};

use crate::{
    components::{CharacterModel, CharacterModelPart},
    render::StaticMeshMaterial,
};

impl CharacterModelPart {
    fn default_bone_id(&self, dummy_bone_offset: usize) -> Option<usize> {
        match *self {
            CharacterModelPart::CharacterFace => Some(4),
            CharacterModelPart::CharacterHair => Some(4),
            CharacterModelPart::Head => Some(dummy_bone_offset + 6),
            CharacterModelPart::FaceItem => Some(dummy_bone_offset + 4),
            CharacterModelPart::Back => Some(dummy_bone_offset + 3),
            _ => None,
        }
    }
}

impl From<ItemType> for CharacterModelPart {
    fn from(item_type: ItemType) -> Self {
        match item_type {
            ItemType::Face => CharacterModelPart::FaceItem,
            ItemType::Head => CharacterModelPart::Head,
            ItemType::Body => CharacterModelPart::Body,
            ItemType::Hands => CharacterModelPart::Hands,
            ItemType::Feet => CharacterModelPart::Feet,
            ItemType::Back => CharacterModelPart::Back,
            ItemType::Weapon => CharacterModelPart::Weapon,
            ItemType::SubWeapon => CharacterModelPart::SubWeapon,
            _ => panic!("Invalid ItemType for CharacterModelPart"),
        }
    }
}

pub struct CharacterModelList {
    skeleton_male: ZmdFile,

    face_male: ZscFile,
    hair_male: ZscFile,
    head_male: ZscFile,
    body_male: ZscFile,
    arms_male: ZscFile,
    feet_male: ZscFile,

    skeleton_female: ZmdFile,
    face_female: ZscFile,
    hair_female: ZscFile,
    head_female: ZscFile,
    body_female: ZscFile,
    arms_female: ZscFile,
    feet_female: ZscFile,

    face_item: ZscFile,
    back: ZscFile,
    weapon: ZscFile,
    sub_weapon: ZscFile,
}

impl CharacterModelList {
    pub fn new(vfs: &VfsIndex) -> Result<CharacterModelList, anyhow::Error> {
        Ok(CharacterModelList {
            skeleton_male: vfs.read_file::<ZmdFile, _>("3DDATA/AVATAR/MALE.ZMD")?,
            face_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MFACE.ZSC")?,
            hair_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MHAIR.ZSC")?,
            head_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MCAP.ZSC")?,
            body_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MBODY.ZSC")?,
            arms_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MARMS.ZSC")?,
            feet_male: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_MFOOT.ZSC")?,
            skeleton_female: vfs.read_file::<ZmdFile, _>("3DDATA/AVATAR/FEMALE.ZMD")?,
            face_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WFACE.ZSC")?,
            hair_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WHAIR.ZSC")?,
            head_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WCAP.ZSC")?,
            body_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WBODY.ZSC")?,
            arms_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WARMS.ZSC")?,
            feet_female: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_WFOOT.ZSC")?,
            face_item: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_FACEIEM.ZSC")?, // Not a typo
            back: vfs.read_file::<ZscFile, _>("3DDATA/AVATAR/LIST_BACK.ZSC")?,
            weapon: vfs.read_file::<ZscFile, _>("3DDATA/WEAPON/LIST_WEAPON.ZSC")?,
            sub_weapon: vfs.read_file::<ZscFile, _>("3DDATA/WEAPON/LIST_SUBWPN.ZSC")?,
        })
    }

    pub fn get_skeleton(&self, gender: CharacterGender) -> &ZmdFile {
        match gender {
            CharacterGender::Male => &self.skeleton_male,
            CharacterGender::Female => &self.skeleton_female,
        }
    }

    pub fn get_model_list(
        &self,
        gender: CharacterGender,
        model_part: CharacterModelPart,
    ) -> &ZscFile {
        match model_part {
            CharacterModelPart::CharacterFace => match gender {
                CharacterGender::Male => &self.face_male,
                CharacterGender::Female => &self.face_female,
            },
            CharacterModelPart::CharacterHair => match gender {
                CharacterGender::Male => &self.hair_male,
                CharacterGender::Female => &self.hair_female,
            },
            CharacterModelPart::FaceItem => &self.face_item,
            CharacterModelPart::Head => match gender {
                CharacterGender::Male => &self.head_male,
                CharacterGender::Female => &self.head_female,
            },
            CharacterModelPart::Body => match gender {
                CharacterGender::Male => &self.body_male,
                CharacterGender::Female => &self.body_female,
            },
            CharacterModelPart::Hands => match gender {
                CharacterGender::Male => &self.arms_male,
                CharacterGender::Female => &self.arms_female,
            },
            CharacterModelPart::Feet => match gender {
                CharacterGender::Male => &self.feet_male,
                CharacterGender::Female => &self.feet_female,
            },
            CharacterModelPart::Back => &self.back,
            CharacterModelPart::Weapon => &self.weapon,
            CharacterModelPart::SubWeapon => &self.sub_weapon,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_character_model(
    commands: &mut Commands,
    model_entity: Entity,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    skinned_mesh_inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
    character_model_list: &CharacterModelList,
    character_info: &CharacterInfo,
    equipment: &Equipment,
) -> (CharacterModel, SkinnedMesh) {
    let skeleton = character_model_list.get_skeleton(character_info.gender);
    let dummy_bone_offset = skeleton.bones.len();
    let skinned_mesh = spawn_skeleton(
        commands,
        model_entity,
        character_model_list.get_skeleton(character_info.gender),
        skinned_mesh_inverse_bindposes_assets,
    );
    let mut model_parts = EnumMap::default();

    for model_part in [
        CharacterModelPart::CharacterFace,
        CharacterModelPart::CharacterHair,
        CharacterModelPart::Head,
        CharacterModelPart::FaceItem,
        CharacterModelPart::Body,
        CharacterModelPart::Hands,
        CharacterModelPart::Feet,
        CharacterModelPart::Back,
        CharacterModelPart::Weapon,
        CharacterModelPart::SubWeapon,
    ] {
        if let Some(model_id) = get_model_part_index(character_info, equipment, model_part) {
            model_parts[model_part] = spawn_model(
                commands,
                model_entity,
                asset_server,
                static_mesh_materials,
                character_model_list.get_model_list(character_info.gender, model_part),
                model_id,
                &skinned_mesh,
                model_part.default_bone_id(dummy_bone_offset),
                dummy_bone_offset,
            );
        }
    }

    (
        CharacterModel {
            gender: character_info.gender,
            model_parts,
            dummy_bone_offset,
        },
        skinned_mesh,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn update_character_equipment(
    commands: &mut Commands,
    model_entity: Entity,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    character_model_list: &CharacterModelList,
    character_model: &mut CharacterModel,
    skinned_mesh: &SkinnedMesh,
    character_info: &CharacterInfo,
    equipment: &Equipment,
) {
    for model_part in [
        CharacterModelPart::CharacterFace,
        CharacterModelPart::CharacterHair,
        CharacterModelPart::Head,
        CharacterModelPart::FaceItem,
        CharacterModelPart::Body,
        CharacterModelPart::Hands,
        CharacterModelPart::Feet,
        CharacterModelPart::Back,
        CharacterModelPart::Weapon,
        CharacterModelPart::SubWeapon,
    ] {
        let model_id = get_model_part_index(character_info, equipment, model_part).unwrap_or(0);

        if model_id != character_model.model_parts[model_part].0 {
            // Despawn previous model
            for &entity in character_model.model_parts[model_part].1.iter() {
                commands.entity(entity).despawn();
            }

            // Spawn new model
            if model_id != 0 {
                character_model.model_parts[model_part] = spawn_model(
                    commands,
                    model_entity,
                    asset_server,
                    static_mesh_materials,
                    character_model_list.get_model_list(character_info.gender, model_part),
                    model_id,
                    skinned_mesh,
                    model_part.default_bone_id(character_model.dummy_bone_offset),
                    character_model.dummy_bone_offset,
                );
            }
        }
    }
}

fn transform_children(skeleton: &ZmdFile, bone_transforms: &mut Vec<Transform>, bone_index: usize) {
    for (child_id, child_bone) in skeleton.bones.iter().enumerate() {
        if child_id == bone_index || child_bone.parent as usize != bone_index {
            continue;
        }

        bone_transforms[child_id] = bone_transforms[bone_index] * bone_transforms[child_id];
        transform_children(skeleton, bone_transforms, child_id);
    }
}

pub fn spawn_skeleton(
    commands: &mut Commands,
    model_entity: Entity,
    skeleton: &ZmdFile,
    skinned_mesh_inverse_bindposes_assets: &mut Assets<SkinnedMeshInverseBindposes>,
) -> SkinnedMesh {
    let mut bind_pose = Vec::with_capacity(skeleton.bones.len());
    let mut bone_entities = Vec::with_capacity(skeleton.bones.len());
    let dummy_bone_offset = skeleton.bones.len();

    for bone in skeleton.bones.iter().chain(skeleton.dummy_bones.iter()) {
        let position = Vec3::new(bone.position.x, bone.position.z, -bone.position.y) / 100.0;

        let rotation = Quat::from_xyzw(
            bone.rotation.x,
            bone.rotation.z,
            -bone.rotation.y,
            bone.rotation.w,
        );

        let transform = Transform::default()
            .with_translation(position)
            .with_rotation(rotation);

        bind_pose.push(transform);

        bone_entities.push(
            commands
                .spawn_bundle((transform, GlobalTransform::default()))
                .id(),
        );
    }

    // Apply parent-child transform hierarchy to calculate bind pose for each bone
    transform_children(skeleton, &mut bind_pose, 0);
    for (dummy_id, dummy_bone) in skeleton.dummy_bones.iter().enumerate() {
        bind_pose[dummy_id + dummy_bone_offset] =
            bind_pose[dummy_id + dummy_bone_offset] * bind_pose[dummy_bone.parent as usize];
    }

    let inverse_bind_pose: Vec<Mat4> = bind_pose
        .iter()
        .map(|x| x.compute_matrix().inverse())
        .collect();

    for (i, bone) in skeleton
        .bones
        .iter()
        .chain(skeleton.dummy_bones.iter())
        .enumerate()
    {
        if let Some(&bone_entity) = bone_entities.get(i) {
            if bone.parent as usize == i {
                commands.entity(model_entity).add_child(bone_entity);
            } else if let Some(&parent_entity) = bone_entities.get(bone.parent as usize) {
                commands.entity(parent_entity).add_child(bone_entity);
            }
        }
    }

    SkinnedMesh {
        inverse_bindposes: skinned_mesh_inverse_bindposes_assets
            .add(SkinnedMeshInverseBindposes::from(inverse_bind_pose)),
        joints: bone_entities,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_model(
    commands: &mut Commands,
    model_entity: Entity,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    model_list: &ZscFile,
    model_id: usize,
    skinned_mesh: &SkinnedMesh,
    default_bone_index: Option<usize>,
    dummy_bone_offset: usize,
) -> (usize, Vec<Entity>) {
    let mut parts = Vec::new();
    let object = if let Some(object) = model_list.objects.get(model_id) {
        object
    } else {
        return (model_id, parts);
    };

    for object_part in object.parts.iter() {
        let mesh_id = object_part.mesh_id as usize;
        let mesh = asset_server.load::<Mesh, _>(model_list.meshes[mesh_id].path());
        let material_id = object_part.material_id as usize;
        let zsc_material = &model_list.materials[material_id];
        let material = static_mesh_materials.add(StaticMeshMaterial {
            base_texture: Some(asset_server.load(zsc_material.path.path())),
            lightmap_texture: None,
            alpha_value: if zsc_material.alpha != 1.0 {
                Some(zsc_material.alpha)
            } else {
                None
            },
            alpha_enabled: zsc_material.alpha_enabled,
            alpha_test: zsc_material.alpha_test,
            two_sided: zsc_material.two_sided,
            z_write_enabled: zsc_material.z_write_enabled,
            z_test_enabled: zsc_material.z_test_enabled,
            specular_enabled: zsc_material.specular_enabled,
            skinned: zsc_material.is_skin,
            ..Default::default()
        });

        let entity = commands
            .spawn_bundle((
                mesh,
                material,
                skinned_mesh.clone(),
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .id();

        let link_bone_entity = if let Some(bone_index) = object_part.bone_index {
            skinned_mesh.joints.get(bone_index as usize).cloned()
        } else if let Some(dummy_index) = object_part.dummy_index {
            skinned_mesh
                .joints
                .get(dummy_index as usize + dummy_bone_offset)
                .cloned()
        } else if let Some(default_bone_index) = default_bone_index {
            skinned_mesh
                .joints
                .get(default_bone_index as usize)
                .cloned()
        } else {
            None
        };
        commands
            .entity(link_bone_entity.unwrap_or(model_entity))
            .add_child(entity);

        parts.push(entity);
    }

    (model_id, parts)
}

fn get_model_part_index(
    character_info: &CharacterInfo,
    equipment: &Equipment,
    model_part: CharacterModelPart,
) -> Option<usize> {
    match model_part {
        CharacterModelPart::CharacterFace => Some(character_info.face as usize),
        CharacterModelPart::CharacterHair => Some(character_info.hair as usize),
        CharacterModelPart::Head => equipment.equipped_items[EquipmentIndex::Head]
            .as_ref()
            .map(|equipment_item| equipment_item.item.item_number),
        CharacterModelPart::FaceItem => equipment.equipped_items[EquipmentIndex::Face]
            .as_ref()
            .map(|equipment_item| equipment_item.item.item_number),
        CharacterModelPart::Body => Some(
            equipment.equipped_items[EquipmentIndex::Body]
                .as_ref()
                .map(|equipment_item| equipment_item.item.item_number)
                .unwrap_or(1),
        ),
        CharacterModelPart::Hands => Some(
            equipment.equipped_items[EquipmentIndex::Hands]
                .as_ref()
                .map(|equipment_item| equipment_item.item.item_number)
                .unwrap_or(1),
        ),
        CharacterModelPart::Feet => Some(
            equipment.equipped_items[EquipmentIndex::Feet]
                .as_ref()
                .map(|equipment_item| equipment_item.item.item_number)
                .unwrap_or(1),
        ),
        CharacterModelPart::Back => equipment.equipped_items[EquipmentIndex::Back]
            .as_ref()
            .map(|equipment_item| equipment_item.item.item_number),
        CharacterModelPart::Weapon => equipment.equipped_items[EquipmentIndex::WeaponRight]
            .as_ref()
            .map(|equipment_item| equipment_item.item.item_number),
        CharacterModelPart::SubWeapon => equipment.equipped_items[EquipmentIndex::WeaponLeft]
            .as_ref()
            .map(|equipment_item| equipment_item.item.item_number),
    }
}
