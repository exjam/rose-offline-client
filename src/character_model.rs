use bevy::{
    math::{Quat, Vec3},
    pbr::{PbrBundle, StandardMaterial},
    prelude::{
        AssetServer, Assets, BuildChildren, Commands, Component, ComputedVisibility, Entity,
        GlobalTransform, Handle, Mesh, Transform, Visibility,
    },
};
use enum_map::{Enum, EnumMap};

use rose_data::{EquipmentIndex, ItemType};
use rose_file_readers::{VfsIndex, ZmdFile, ZscFile};
use rose_game_common::components::{CharacterGender, CharacterInfo, Equipment};

use crate::render::StaticMeshMaterial;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Enum)]
pub enum CharacterModelPart {
    CharacterFace,
    CharacterHair,
    Head,
    FaceItem,
    Body,
    Hands,
    Feet,
    Back,
    Weapon,
    SubWeapon,
}

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

pub struct LoadedSkeleton {
    pub bones: Vec<Entity>,
    pub dummy_bone_offset: usize,
}

fn spawn_skeleton(
    commands: &mut Commands,
    skeleton: &ZmdFile,
    bone_visualisation: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
) -> LoadedSkeleton {
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

        if let Some((bone_mesh, bone_material)) = &bone_visualisation {
            bone_entities.push(
                commands
                    .spawn_bundle(PbrBundle {
                        mesh: bone_mesh.clone(),
                        material: bone_material.clone(),
                        transform,
                        ..Default::default()
                    })
                    .id(),
            );
        } else {
            bone_entities.push(
                commands
                    .spawn_bundle((transform, GlobalTransform::default()))
                    .id(),
            );
        }
    }

    for (i, bone) in skeleton
        .bones
        .iter()
        .chain(skeleton.dummy_bones.iter())
        .enumerate()
    {
        if bone.parent as usize == i {
            continue;
        }

        if let Some(&bone_entity) = bone_entities.get(i) {
            if let Some(&parent_entity) = bone_entities.get(bone.parent as usize) {
                commands.entity(parent_entity).add_child(bone_entity);
            }
        }
    }

    LoadedSkeleton {
        bones: bone_entities,
        dummy_bone_offset,
    }
}

fn spawn_model(
    commands: &mut Commands,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    model_list: &ZscFile,
    model_id: usize,
    skeleton: &LoadedSkeleton,
    default_bone_id: usize,
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
            ..Default::default()
        });

        let entity = commands
            .spawn_bundle((
                mesh,
                material,
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ))
            .id();

        let link_bone_id = object_part.bone_index.unwrap_or_else(|| {
            object_part
                .dummy_index
                .map(|x| x + skeleton.dummy_bone_offset as u16)
                .unwrap_or(default_bone_id as u16)
        }) as usize;

        if let Some(&parent_entity) = skeleton.bones.get(link_bone_id as usize) {
            commands.entity(parent_entity).add_child(entity);
        }

        parts.push(entity);
    }

    (model_id, parts)
}

#[derive(Component)]
pub struct CharacterModel {
    pub gender: CharacterGender,
    pub skeleton: LoadedSkeleton,
    pub model_parts: EnumMap<CharacterModelPart, (usize, Vec<Entity>)>,
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

pub fn spawn_character_model(
    commands: &mut Commands,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    character_model_list: &CharacterModelList,
    character_info: &CharacterInfo,
    equipment: &Equipment,
    bone_visualisation: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
) -> CharacterModel {
    let skeleton = spawn_skeleton(
        commands,
        character_model_list.get_skeleton(character_info.gender),
        bone_visualisation,
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
                asset_server,
                static_mesh_materials,
                character_model_list.get_model_list(character_info.gender, model_part),
                model_id,
                &skeleton,
                model_part
                    .default_bone_id(skeleton.dummy_bone_offset)
                    .unwrap_or(0),
            );
        }
    }

    CharacterModel {
        gender: character_info.gender,
        skeleton,
        model_parts,
    }
}

pub fn update_character_equipment(
    commands: &mut Commands,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    character_model_list: &CharacterModelList,
    character_model: &mut CharacterModel,
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
                    asset_server,
                    static_mesh_materials,
                    character_model_list.get_model_list(character_info.gender, model_part),
                    model_id,
                    &character_model.skeleton,
                    model_part
                        .default_bone_id(character_model.skeleton.dummy_bone_offset)
                        .unwrap_or(0),
                );
            }
        }
    }
}
