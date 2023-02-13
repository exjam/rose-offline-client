use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use arrayvec::ArrayVec;
use bevy::{
    asset::{AssetLoader, BoxedFuture, LoadContext, LoadState, LoadedAsset},
    ecs::system::SystemParam,
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    math::{Quat, Vec2, Vec3},
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{
        AssetServer, Assets, Commands, ComputedVisibility, Entity, EventReader, EventWriter,
        GlobalTransform, Handle, HandleUntyped, Local, Mesh, Res, ResMut, Transform, Visibility,
    },
    reflect::TypeUuid,
    render::{
        mesh::{Indices, PrimitiveTopology},
        view::NoFrustumCulling,
    },
    tasks::IoTaskPool,
};
use bevy_rapier3d::prelude::{
    AsyncCollider, Collider, CollisionGroups, ComputedColliderShape, RigidBody,
};
use thiserror::Error;

use rose_data::{SkyboxData, WarpGateId, ZoneId, ZoneList};
use rose_file_readers::{
    HimFile, IfoFile, IfoObject, LitFile, LitObject, RoseFile, RoseFileReader, StbFile, TilFile,
    ZonFile, ZonTileRotation, ZscCollisionFlags, ZscEffectType, ZscFile,
};

use crate::{
    components::{
        ActiveMotion, ColliderParent, EventObject, NightTimeEffect, WarpObject, Zone, ZoneObject,
        ZoneObjectAnimatedObject, ZoneObjectId, ZoneObjectPart, ZoneObjectTerrain,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_INSPECTABLE,
        COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
        COLLISION_GROUP_ZONE_WARP_OBJECT, COLLISION_GROUP_ZONE_WATER,
    },
    effect_loader::spawn_effect,
    events::{LoadZoneEvent, ZoneEvent},
    render::{
        EffectMeshMaterial, ObjectMaterial, ObjectMaterialBlend, ParticleMaterial,
        RgbTextureLoader, SkyMaterial, TerrainMaterial, TextureArray, TextureArrayBuilder,
        WaterMaterial, MESH_ATTRIBUTE_UV_1, TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
    },
    resources::{CurrentZone, DebugInspector, GameData},
    VfsResource,
};

#[derive(Error, Debug)]
pub enum ZoneLoadError {
    #[error("Invalid Zone Id")]
    InvalidZoneId,
}

pub struct ZoneLoaderBlock {
    pub block_x: usize,
    pub block_y: usize,
    pub him: HimFile,
    pub til: Option<TilFile>,
    pub ifo: Option<IfoFile>,
    pub lit_cnst: Option<LitFile>,
    pub lit_deco: Option<LitFile>,
}

#[derive(TypeUuid)]
#[uuid = "596e2c17-f2dd-4276-8df4-1e94dc0d056b"]
pub struct ZoneLoaderAsset {
    pub zone_id: ZoneId,
    pub zone_path: PathBuf,
    pub zon: ZonFile,
    pub zsc_cnst: ZscFile,
    pub zsc_deco: ZscFile,
    pub blocks: Vec<Option<Box<ZoneLoaderBlock>>>,
}

impl ZoneLoaderAsset {
    pub fn get_terrain_height(&self, x: f32, y: f32) -> f32 {
        let block_x = x / (16.0 * self.zon.grid_per_patch * self.zon.grid_size);
        let block_y = 65.0 - (y / (16.0 * self.zon.grid_per_patch * self.zon.grid_size));

        if let Some(heightmap) = self
            .blocks
            .get(block_x.clamp(0.0, 64.0) as usize + block_y.clamp(0.0, 64.0) as usize * 64)
            .and_then(|block| block.as_ref())
            .map(|block| &block.him)
        {
            let tile_x = (heightmap.width - 1) as f32 * block_x.fract();
            let tile_y = (heightmap.height - 1) as f32 * block_y.fract();

            let tile_index_x = tile_x as i32;
            let tile_index_y = tile_y as i32;

            let height_00 = heightmap.get_clamped(tile_index_x, tile_index_y);
            let height_01 = heightmap.get_clamped(tile_index_x, tile_index_y + 1);
            let height_10 = heightmap.get_clamped(tile_index_x + 1, tile_index_y);
            let height_11 = heightmap.get_clamped(tile_index_x + 1, tile_index_y + 1);

            let weight_x = tile_x.fract();
            let weight_y = tile_y.fract();

            let height_y0 = height_00 * (1.0 - weight_x) + height_10 * weight_x;
            let height_y1 = height_01 * (1.0 - weight_x) + height_11 * weight_x;

            height_y0 * (1.0 - weight_y) + height_y1 * weight_y
        } else {
            0.0
        }
    }

    pub fn get_tile_index(&self, x: f32, y: f32) -> usize {
        let block_x = x / (16.0 * self.zon.grid_per_patch * self.zon.grid_size);
        let block_y = 65.0 - (y / (16.0 * self.zon.grid_per_patch * self.zon.grid_size));

        if let Some(tilemap) = self
            .blocks
            .get(block_x.clamp(0.0, 64.0) as usize + block_y.clamp(0.0, 64.0) as usize * 64)
            .and_then(|block| block.as_ref())
            .and_then(|block| block.til.as_ref())
        {
            let tile_x = tilemap.width as f32 * block_x.fract();
            let tile_y = tilemap.height as f32 * block_y.fract();

            let tile_index_x = tile_x as usize;
            let tile_index_y = tile_y as usize;

            let tile_index = tilemap.get_clamped(tile_index_x, tile_index_y) as usize;

            if let Some(tile_info) = self.zon.tiles.get(tile_index) {
                (tile_info.layer2 + tile_info.offset2) as usize
            } else {
                0
            }
        } else {
            0
        }
    }
}

pub struct ZoneLoader {
    pub zone_list: Arc<ZoneList>,
}

impl AssetLoader for ZoneLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            load_zone(self, ZoneId::new(bytes[0] as u16).unwrap(), load_context).await
        })
    }

    fn extensions(&self) -> &[&str] {
        &["zone_loader"]
    }
}

async fn load_zone<'a, 'b>(
    zone_loader: &'a ZoneLoader,
    zone_id: ZoneId,
    load_context: &'a mut LoadContext<'b>,
) -> Result<(), anyhow::Error> {
    let zone_list_entry = zone_loader
        .zone_list
        .get_zone(zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;

    let zon: ZonFile = RoseFile::read(
        RoseFileReader::from(
            &load_context
                .read_asset_bytes(zone_list_entry.zon_file_path.path())
                .await?,
        ),
        &Default::default(),
    )?;
    let zsc_cnst: ZscFile = RoseFile::read(
        RoseFileReader::from(
            &load_context
                .read_asset_bytes(zone_list_entry.zsc_cnst_path.path())
                .await?,
        ),
        &Default::default(),
    )?;
    let zsc_deco: ZscFile = RoseFile::read(
        RoseFileReader::from(
            &load_context
                .read_asset_bytes(zone_list_entry.zsc_deco_path.path())
                .await?,
        ),
        &Default::default(),
    )?;
    let zone_path = zone_list_entry
        .zon_file_path
        .path()
        .parent()
        .unwrap_or_else(|| Path::new(""));

    let zone_blocks_iterator = IoTaskPool::get()
        .scope(|scope| {
            for block_y in 0..64 {
                for block_x in 0..64 {
                    let load_context: &LoadContext = load_context;

                    scope.spawn(async move {
                        load_block_files(load_context, zone_path, block_x, block_y).await
                    });
                }
            }
        })
        .into_iter()
        .filter_map(|result| result.ok());

    let mut blocks = Vec::new();
    blocks.resize_with(64 * 64, || None);
    for block in zone_blocks_iterator {
        let index = block.block_x + block.block_y * 64;
        blocks[index] = Some(block);
    }

    load_context.set_default_asset(LoadedAsset::new(ZoneLoaderAsset {
        zone_path: zone_path.into(),
        zone_id,
        zon,
        zsc_cnst,
        zsc_deco,
        blocks,
    }));
    Ok(())
}

async fn load_block_files<'a>(
    load_context: &LoadContext<'a>,
    zone_path: &Path,
    block_x: usize,
    block_y: usize,
) -> Result<Box<ZoneLoaderBlock>, anyhow::Error> {
    let him = RoseFile::read(
        RoseFileReader::from(
            &load_context
                .read_asset_bytes(zone_path.join(format!("{}_{}.HIM", block_x, block_y)))
                .await?,
        ),
        &Default::default(),
    )?;

    let til = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!("{}_{}.TIL", block_x, block_y)))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let ifo = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!("{}_{}.IFO", block_x, block_y)))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let lit_cnst = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!(
            "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
            block_x, block_y
        )))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    let lit_deco = if let Ok(data) = load_context
        .read_asset_bytes(zone_path.join(format!(
            "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
            block_x, block_y
        )))
        .await
    {
        RoseFile::read(RoseFileReader::from(&data), &Default::default()).ok()
    } else {
        None
    };

    Ok(Box::new(ZoneLoaderBlock {
        block_x,
        block_y,
        til,
        him,
        ifo,
        lit_cnst,
        lit_deco,
    }))
}

#[derive(SystemParam)]
pub struct SpawnZoneParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: Res<'w, AssetServer>,
    pub game_data: Res<'w, GameData>,
    pub vfs_resource: Res<'w, VfsResource>,
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub sky_materials: ResMut<'w, Assets<SkyMaterial>>,
    pub terrain_materials: ResMut<'w, Assets<TerrainMaterial>>,
    pub effect_mesh_materials: ResMut<'w, Assets<EffectMeshMaterial>>,
    pub particle_materials: ResMut<'w, Assets<ParticleMaterial>>,
    pub object_materials: ResMut<'w, Assets<ObjectMaterial>>,
    pub water_materials: ResMut<'w, Assets<WaterMaterial>>,
    pub texture_arrays: ResMut<'w, Assets<TextureArray>>,
}

pub struct CachedZone {
    pub data_handle: Handle<ZoneLoaderAsset>,
    pub spawned_entity: Option<Entity>,
}

pub enum LoadingZoneState {
    Loading,
    Spawned,
}

pub struct LoadingZone {
    pub state: LoadingZoneState,
    pub handle: Handle<ZoneLoaderAsset>,
    pub despawn_other_zones: bool,
    pub zone_assets: Vec<HandleUntyped>,
    pub ready_frames: usize,
}

#[derive(Default)]
pub struct ZoneLoaderCache {
    pub cache: Vec<Option<CachedZone>>,
}

pub fn zone_loader_system(
    mut zone_loader_cache: Local<ZoneLoaderCache>,
    mut loading_zones: Local<Vec<LoadingZone>>,
    mut load_zone_events: EventReader<LoadZoneEvent>,
    mut zone_events: EventWriter<ZoneEvent>,
    mut spawn_zone_params: SpawnZoneParams,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    mut debug_inspector_state: ResMut<DebugInspector>,
) {
    if zone_loader_cache.cache.is_empty() {
        zone_loader_cache
            .cache
            .resize_with(spawn_zone_params.game_data.zone_list.len(), || None);
    }

    for event in load_zone_events.iter() {
        let zone_index = event.id.get() as usize;

        if zone_loader_cache.cache[zone_index].is_none() {
            zone_loader_cache.cache[zone_index] = Some(CachedZone {
                data_handle: spawn_zone_params
                    .asset_server
                    .load(format!("{}.zone_loader", zone_index)),
                spawned_entity: None,
            });
        } else if let Some(zone_entity) = zone_loader_cache.cache[zone_index]
            .as_ref()
            .and_then(|cached_zone| cached_zone.spawned_entity)
        {
            // Zone is already spawned
            zone_events.send(ZoneEvent::Loaded(event.id));
            debug_inspector_state.entity = Some(zone_entity);
            continue;
        }

        let cached_zone = zone_loader_cache.cache[zone_index].as_ref().unwrap();
        loading_zones.push(LoadingZone {
            state: LoadingZoneState::Loading,
            handle: cached_zone.data_handle.clone(),
            despawn_other_zones: event.despawn_other_zones,
            zone_assets: Vec::default(),
            ready_frames: 0,
        });
    }

    let mut index = 0;
    while index < loading_zones.len() {
        let loading_zone = &mut loading_zones[index];

        match loading_zone.state {
            LoadingZoneState::Loading => {
                match spawn_zone_params
                    .asset_server
                    .get_load_state(&loading_zone.handle)
                {
                    LoadState::NotLoaded | LoadState::Loading => {
                        index += 1;
                    }
                    LoadState::Loaded => {
                        if let Some(zone_data) = zone_loader_assets.get(&loading_zone.handle) {
                            // Despawn other zones
                            if loading_zone.despawn_other_zones {
                                for cached_zone in zone_loader_cache
                                    .cache
                                    .iter_mut()
                                    .filter_map(|x| x.as_mut())
                                {
                                    if let Some(spawned_entity) = cached_zone.spawned_entity.take()
                                    {
                                        spawn_zone_params
                                            .commands
                                            .entity(spawned_entity)
                                            .despawn_recursive();
                                    }
                                }

                                spawn_zone_params.commands.remove_resource::<CurrentZone>();
                            }

                            // Spawn next zone
                            if let Ok((zone_entity, loading_assets)) =
                                spawn_zone(&mut spawn_zone_params, zone_data)
                            {
                                zone_loader_cache.cache[zone_data.zone_id.get() as usize] =
                                    Some(CachedZone {
                                        data_handle: loading_zone.handle.clone(),
                                        spawned_entity: Some(zone_entity),
                                    });

                                spawn_zone_params.commands.insert_resource(CurrentZone {
                                    id: zone_data.zone_id,
                                    handle: loading_zone.handle.clone(),
                                });

                                debug_inspector_state.entity = Some(zone_entity);
                                loading_zone.zone_assets = loading_assets;
                            }

                            if loading_zone.zone_assets.is_empty() {
                                zone_events.send(ZoneEvent::Loaded(zone_data.zone_id));
                                loading_zones.remove(index);
                            } else {
                                loading_zone.state = LoadingZoneState::Spawned;
                            }
                        } else {
                            index += 1;
                        }
                    }
                    LoadState::Unloaded | LoadState::Failed => {
                        loading_zones.remove(index);
                    }
                }
            }
            LoadingZoneState::Spawned => {
                let is_loading = loading_zone.zone_assets.iter().any(|handle| {
                    matches!(
                        spawn_zone_params.asset_server.get_load_state(handle),
                        LoadState::NotLoaded | LoadState::Loading
                    )
                });

                if is_loading {
                    index += 1;
                } else if let Some(zone_data) = zone_loader_assets.get(&loading_zone.handle) {
                    // The physics system will take 2 frames to initialise colliders properly
                    loading_zone.ready_frames += 1;

                    if loading_zone.ready_frames == 2 {
                        zone_events.send(ZoneEvent::Loaded(zone_data.zone_id));
                        loading_zones.remove(index);
                    } else {
                        index += 1;
                    }
                } else {
                    index += 1;
                }
            }
        }
    }
}

pub fn spawn_zone(
    params: &mut SpawnZoneParams,
    zone_data: &ZoneLoaderAsset,
) -> Result<(Entity, Vec<HandleUntyped>), anyhow::Error> {
    let SpawnZoneParams {
        commands,
        asset_server,
        game_data,
        vfs_resource,
        meshes,
        sky_materials,
        terrain_materials,
        effect_mesh_materials,
        particle_materials,
        object_materials,
        water_materials,
        texture_arrays,
    } = params;

    let zone_list_entry = game_data
        .zone_list
        .get_zone(zone_data.zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;

    let tilemap_texture_array = {
        let mut tilemap_texture_array_builder = TextureArrayBuilder::new();
        for path in zone_data.zon.tile_textures.iter() {
            if path == "end" {
                break;
            }

            tilemap_texture_array_builder.add(path.clone());
        }
        texture_arrays.add(tilemap_texture_array_builder.build(asset_server))
    };

    let water_material = {
        let mut water_texture_array_builder = TextureArrayBuilder::new();
        for i in 1..=25 {
            water_texture_array_builder.add(format!("3DDATA/JUNON/WATER/OCEAN01_{:02}.DDS", i));
        }

        water_materials.add(WaterMaterial {
            water_texture_array: texture_arrays
                .add(water_texture_array_builder.build(asset_server)),
        })
    };

    let mut zone_loading_assets: Vec<HandleUntyped> = Vec::default();
    let zone_entity = commands
        .spawn((
            Zone {
                id: zone_data.zone_id,
            },
            Visibility::default(),
            ComputedVisibility::default(),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();

    if let Some(skybox_data) = zone_list_entry
        .skybox_id
        .and_then(|skybox_id| game_data.skybox.get_skybox_data(skybox_id))
    {
        let skybox_entity = spawn_skybox(commands, asset_server, sky_materials, skybox_data);
        commands.entity(zone_entity).add_child(skybox_entity);
    }

    for block_y in 0..64 {
        for block_x in 0..64 {
            if let Some(block_data) = zone_data.blocks[block_x + block_y * 64].as_ref() {
                let terrain_entity = spawn_terrain(
                    commands,
                    asset_server,
                    meshes,
                    terrain_materials,
                    tilemap_texture_array.clone(),
                    zone_data,
                    block_data,
                );
                commands.entity(zone_entity).add_child(terrain_entity);

                if let Some(ifo) = block_data.ifo.as_ref() {
                    let lightmap_path = zone_data
                        .zone_path
                        .join(format!("{}_{}/LIGHTMAP/", block_x, block_y));

                    for (plane_start, plane_end) in ifo.water_planes.iter() {
                        let water_entity = spawn_water(
                            commands,
                            meshes,
                            &water_material,
                            ifo.water_size,
                            Vec3::new(plane_start.x, plane_start.y, plane_start.z),
                            Vec3::new(plane_end.x, plane_end.y, plane_end.z),
                        );
                        commands.entity(zone_entity).add_child(water_entity);
                    }

                    for (ifo_object_id, event_object) in ifo.event_objects.iter().enumerate() {
                        let event_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            &game_data.zsc_event_object,
                            &lightmap_path,
                            None,
                            &event_object.object,
                            ifo_object_id,
                            event_object.object.object_id as usize,
                            ZoneObject::EventObject,
                            ZoneObject::EventObjectPart,
                            COLLISION_GROUP_ZONE_EVENT_OBJECT,
                        );

                        commands.entity(event_entity).insert(EventObject::new(
                            event_object.quest_trigger_name.clone(),
                            event_object.script_function_name.clone(),
                        ));
                        commands.entity(zone_entity).add_child(event_entity);
                    }

                    for (ifo_object_id, warp_object) in ifo.warps.iter().enumerate() {
                        let warp_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            &game_data.zsc_special_object,
                            &lightmap_path,
                            None,
                            warp_object,
                            ifo_object_id,
                            1,
                            ZoneObject::WarpObject,
                            ZoneObject::WarpObjectPart,
                            COLLISION_GROUP_ZONE_WARP_OBJECT,
                        );

                        commands
                            .entity(warp_entity)
                            .insert(WarpObject::new(WarpGateId::new(warp_object.warp_id)));
                        commands.entity(zone_entity).add_child(warp_entity);
                    }

                    for (ifo_object_id, object_instance) in ifo.cnst_objects.iter().enumerate() {
                        let lit_object = block_data.lit_cnst.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == ifo_object_id + 1)
                        });

                        let object_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            &zone_data.zsc_cnst,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            ifo_object_id,
                            object_instance.object_id as usize,
                            ZoneObject::CnstObject,
                            ZoneObject::CnstObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                    }

                    for (ifo_object_id, object_instance) in ifo.deco_objects.iter().enumerate() {
                        let lit_object = block_data.lit_deco.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == ifo_object_id + 1)
                        });

                        let object_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            object_materials.as_mut(),
                            &zone_data.zsc_deco,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            ifo_object_id,
                            object_instance.object_id as usize,
                            ZoneObject::DecoObject,
                            ZoneObject::DecoObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                    }

                    for object_instance in ifo.animated_objects.iter() {
                        let object_entity = spawn_animated_object(
                            commands,
                            asset_server,
                            object_materials.as_mut(),
                            &game_data.stb_morph_object,
                            object_instance,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                    }
                }
            }
        }
    }

    Ok((zone_entity, zone_loading_assets))
}

const SKYBOX_MODEL_SCALE: f32 = 10.0;

fn spawn_skybox(
    commands: &mut Commands,
    asset_server: &AssetServer,
    sky_materials: &mut Assets<SkyMaterial>,
    skybox_data: &SkyboxData,
) -> Entity {
    commands
        .spawn((
            asset_server.load::<Mesh, _>(skybox_data.mesh.path()),
            sky_materials.add(SkyMaterial {
                texture_day: Some(asset_server.load(RgbTextureLoader::convert_path(
                    skybox_data.texture_day.path(),
                ))),
                texture_night: Some(asset_server.load(RgbTextureLoader::convert_path(
                    skybox_data.texture_night.path(),
                ))),
            }),
            Transform::from_scale(Vec3::splat(SKYBOX_MODEL_SCALE)),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            NoFrustumCulling,
        ))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn spawn_terrain(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    terrain_materials: &mut Assets<TerrainMaterial>,
    tilemap_texture_array: Handle<TextureArray>,
    zone_data: &ZoneLoaderAsset,
    block_data: &ZoneLoaderBlock,
) -> Entity {
    let offset_x = 160.0 * block_data.block_x as f32;
    let offset_y = 160.0 * (65.0 - block_data.block_y as f32);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs_lightmap = Vec::new();
    let mut uvs_tile = Vec::new();
    let mut indices = Vec::new();
    let mut tile_ids = Vec::new();

    let tilemap = block_data.til.as_ref();
    let heightmap = &block_data.him;

    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile = &zone_data.zon.tiles[tilemap
                .map(|tilemap| tilemap.get_clamped(tile_x, tile_y) as usize)
                .unwrap_or(0)];
            let tile_array_index1 = tile.layer1 + tile.offset1;
            let tile_array_index2 = tile.layer2 + tile.offset2;
            let tile_rotation = match tile.rotation {
                ZonTileRotation::FlipHorizontal => 2,
                ZonTileRotation::FlipVertical => 3,
                ZonTileRotation::Flip => 4,
                ZonTileRotation::Clockwise90 => 5,
                ZonTileRotation::CounterClockwise90 => 6,
                _ => 0,
            };
            let tile_indices_base = positions.len() as u16;
            let tile_offset_x = tile_x as f32 * 4.0 * 2.5;
            let tile_offset_y = tile_y as f32 * 4.0 * 2.5;

            for y in 0..5 {
                for x in 0..5 {
                    let heightmap_x = x + tile_x as i32 * 4;
                    let heightmap_y = y + tile_y as i32 * 4;
                    let height = heightmap.get_clamped(heightmap_x, heightmap_y) / 100.0;
                    let height_l = heightmap.get_clamped(heightmap_x - 1, heightmap_y) / 100.0;
                    let height_r = heightmap.get_clamped(heightmap_x + 1, heightmap_y) / 100.0;
                    let height_t = heightmap.get_clamped(heightmap_x, heightmap_y - 1) / 100.0;
                    let height_b = heightmap.get_clamped(heightmap_x, heightmap_y + 1) / 100.0;
                    let normal = Vec3::new(
                        (height_l - height_r) / 2.0,
                        1.0,
                        (height_t - height_b) / 2.0,
                    )
                    .normalize();

                    positions.push([
                        tile_offset_x + x as f32 * 2.5,
                        height,
                        tile_offset_y + y as f32 * 2.5,
                    ]);
                    normals.push([normal.x, normal.y, normal.z]);
                    uvs_tile.push([x as f32 / 4.0, y as f32 / 4.0]);
                    uvs_lightmap.push([
                        (tile_x as f32 * 4.0 + x as f32) / 64.0,
                        (tile_y as f32 * 4.0 + y as f32) / 64.0,
                    ]);
                    tile_ids.push([
                        tile_array_index1 as i32,
                        tile_array_index2 as i32,
                        tile_rotation,
                    ]);
                }
            }

            for y in 0..(5 - 1) {
                for x in 0..(5 - 1) {
                    let start = tile_indices_base + y * 5 + x;
                    indices.push(start);
                    indices.push(start + 5);
                    indices.push(start + 1);

                    indices.push(start + 1);
                    indices.push(start + 5);
                    indices.push(start + 1 + 5);
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U16(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
    mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, uvs_tile);
    mesh.insert_attribute(TERRAIN_MESH_ATTRIBUTE_TILE_INFO, tile_ids);

    let mut collider_verts = Vec::new();
    let mut collider_indices = Vec::new();

    for y in 0..heightmap.height as i32 {
        for x in 0..heightmap.width as i32 {
            collider_verts.push(
                [
                    x as f32 * 2.5,
                    heightmap.get_clamped(x, y) / 100.0,
                    y as f32 * 2.5,
                ]
                .into(),
            );
        }
    }

    for y in 0..(heightmap.height - 1) {
        for x in 0..(heightmap.width - 1) {
            let start = y * heightmap.width + x;
            collider_indices.push([start, start + heightmap.width, start + 1]);
            collider_indices.push([
                start + 1,
                start + heightmap.width,
                start + 1 + heightmap.width,
            ]);
        }
    }

    let material = terrain_materials.add(TerrainMaterial {
        lightmap_texture: asset_server.load(format!(
            "{}/{1:}_{2:}/{1:}_{2:}_PLANELIGHTINGMAP.DDS.rgb_texture",
            zone_data.zone_path.to_str().unwrap(),
            block_data.block_x,
            block_data.block_y,
        )),
        tilemap_texture_array,
    });

    commands
        .spawn((
            ZoneObject::Terrain(ZoneObjectTerrain {
                block_x: block_data.block_x as u32,
                block_y: block_data.block_y as u32,
            }),
            meshes.add(mesh),
            material,
            Transform::from_xyz(offset_x, 0.0, -offset_y),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            NotShadowCaster,
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices),
            CollisionGroups::new(
                COLLISION_GROUP_ZONE_TERRAIN,
                COLLISION_FILTER_INSPECTABLE
                    | COLLISION_FILTER_COLLIDABLE
                    | COLLISION_GROUP_PHYSICS_TOY
                    | COLLISION_FILTER_MOVEABLE
                    | COLLISION_FILTER_CLICKABLE,
            ),
        ))
        .id()
}

fn spawn_water(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_material: &Handle<WaterMaterial>,
    water_size: f32,
    plane_start: Vec3,
    plane_end: Vec3,
) -> Entity {
    let start = Vec3::new(
        5200.0 + plane_start.x / 100.0,
        plane_start.y / 100.0,
        -(5200.0 + plane_start.z / 100.0),
    );
    let end = Vec3::new(
        5200.0 + plane_end.x / 100.0,
        plane_end.y / 100.0,
        -(5200.0 + plane_end.z / 100.0),
    );
    let uv_x = (end.x - start.x) / (water_size / 100.0);
    let uv_y = (end.z - start.z) / (water_size / 100.0);

    let vertices = [
        ([start.x, start.y, end.z], [0.0, 1.0, 0.0], [uv_x, uv_y]),
        ([start.x, start.y, start.z], [0.0, 1.0, 0.0], [uv_x, 0.0]),
        ([end.x, start.y, start.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([end.x, start.y, end.z], [0.0, 1.0, 0.0], [0.0, uv_y]),
    ];
    let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);
    let collider_indices = vec![[0, 2, 1], [0, 3, 2]];

    let mut collider_verts = Vec::new();
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    for (position, normal, uv) in &vertices {
        collider_verts.push((*position).into());
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    commands
        .spawn((
            ZoneObject::Water,
            meshes.add(mesh),
            water_material.clone(),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            NotShadowCaster,
            NotShadowReceiver,
            RigidBody::Fixed,
            Collider::trimesh(collider_verts, collider_indices),
            CollisionGroups::new(COLLISION_GROUP_ZONE_WATER, COLLISION_FILTER_INSPECTABLE),
        ))
        .id()
}

fn spawn_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    zone_loading_assets: &mut Vec<HandleUntyped>,
    vfs_resource: &VfsResource,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    particle_materials: &mut Assets<ParticleMaterial>,
    object_materials: &mut Assets<ObjectMaterial>,
    zsc: &ZscFile,
    lightmap_path: &Path,
    lit_object: Option<&LitObject>,
    object_instance: &IfoObject,
    ifo_object_id: usize,
    zsc_object_id: usize,
    object_type: fn(ZoneObjectId) -> ZoneObject,
    part_object_type: fn(ZoneObjectPart) -> ZoneObject,
    collision_group: bevy_rapier3d::prelude::Group,
) -> Entity {
    let object = &zsc.objects[zsc_object_id];
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mut material_cache: Vec<Option<Handle<ObjectMaterial>>> = vec![None; zsc.materials.len()];
    let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];

    let mut part_entities: ArrayVec<Entity, 256> = ArrayVec::new();
    let mut object_entity_commands = commands.spawn((
        object_type(ZoneObjectId {
            ifo_object_id,
            zsc_object_id,
        }),
        object_transform,
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
        RigidBody::Fixed,
    ));

    let object_entity = object_entity_commands.id();

    object_entity_commands.with_children(|object_commands| {
        for (part_index, object_part) in object.parts.iter().enumerate() {
            let part_transform = Transform::default()
                .with_translation(
                    Vec3::new(
                        object_part.position.x,
                        object_part.position.z,
                        -object_part.position.y,
                    ) / 100.0,
                )
                .with_rotation(Quat::from_xyzw(
                    object_part.rotation.x,
                    object_part.rotation.z,
                    -object_part.rotation.y,
                    object_part.rotation.w,
                ))
                .with_scale(Vec3::new(
                    object_part.scale.x,
                    object_part.scale.z,
                    object_part.scale.y,
                ));

            let mesh_id = object_part.mesh_id as usize;
            let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
                let handle = asset_server.load(zsc.meshes[mesh_id].path());
                mesh_cache.insert(mesh_id, Some(handle.clone()));
                handle
            });
            zone_loading_assets.push(mesh.clone_untyped());
            let lit_part = lit_object.and_then(|lit_object| {
                for part in lit_object.parts.iter() {
                    if part_index == part.object_part_index as usize {
                        return Some(part);
                    }
                }

                lit_object.parts.get(part_index)
            });
            let lightmap_texture = lit_part.map(|lit_part| {
                asset_server.load(RgbTextureLoader::convert_path(
                    &lightmap_path.join(&lit_part.filename),
                ))
            });
            let (lightmap_uv_offset, lightmap_uv_scale) = lit_part
                .map(|lit_part| {
                    let scale = 1.0 / lit_part.parts_per_row as f32;
                    (
                        Vec2::new(
                            (lit_part.part_index % lit_part.parts_per_row) as f32,
                            (lit_part.part_index / lit_part.parts_per_row) as f32,
                        ),
                        scale,
                    )
                })
                .unwrap_or((Vec2::new(0.0, 0.0), 1.0));

            let material_id = object_part.material_id as usize;
            let material = material_cache[material_id].clone().unwrap_or_else(|| {
                let zsc_material = &zsc.materials[material_id];
                let handle = object_materials.add(ObjectMaterial {
                    base_texture: Some(
                        asset_server.load(RgbTextureLoader::convert_path(zsc_material.path.path())),
                    ),
                    lightmap_texture,
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
                    blend: zsc_material.blend_mode.into(),
                    glow: zsc_material.glow.map(|x| x.into()),
                    skinned: zsc_material.is_skin,
                    lightmap_uv_offset,
                    lightmap_uv_scale,
                });

                material_cache.insert(material_id, Some(handle.clone()));
                handle
            });

            let mut collision_filter = COLLISION_FILTER_INSPECTABLE;

            if object_part.collision_shape.is_some() {
                if collision_group != COLLISION_GROUP_ZONE_EVENT_OBJECT
                    && collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT
                    && !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::HEIGHT_ONLY)
                {
                    collision_filter |= COLLISION_FILTER_COLLIDABLE | COLLISION_GROUP_PHYSICS_TOY;
                }

                if collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT {
                    if !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE)
                    {
                        collision_filter |= COLLISION_FILTER_CLICKABLE;
                    }

                    if !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_MOVEABLE)
                    {
                        collision_filter |= COLLISION_FILTER_MOVEABLE;
                    }
                }
            }

            let mut part_commands = object_commands.spawn((
                part_object_type(ZoneObjectPart {
                    ifo_object_id,
                    zsc_object_id,
                    zsc_part_id: part_index,
                    mesh_path: zsc.meshes[mesh_id].path().to_string_lossy().into(),
                    // collision_shape.is_none(): cannot be hit with any raycast
                    // collision_shape.is_some(): can be hit with forward raycast
                    collision_shape: (&object_part.collision_shape).into(),
                    // collision_not_moveable: does not hit downwards ray cast, but can hit forwards ray cast
                    collision_not_moveable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_MOVEABLE),
                    // collision_not_pickable: can not be clicked on with mouse
                    collision_not_pickable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE),
                    // collision_height_only: ?
                    collision_height_only: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::HEIGHT_ONLY),
                    // collision_no_camera: does not collide with camera
                    collision_no_camera: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_CAMERA_COLLISION),
                }),
                mesh.clone(),
                material,
                part_transform,
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
                NotShadowCaster,
                ColliderParent::new(object_entity),
                AsyncCollider {
                    handle: mesh,
                    shape: ComputedColliderShape::TriMesh,
                },
                CollisionGroups::new(collision_group, collision_filter),
            ));

            let active_motion = object_part.animation_path.as_ref().map(|animation_path| {
                ActiveMotion::new_repeating(asset_server.load(animation_path.path()))
            });
            if let Some(active_motion) = active_motion {
                part_commands.insert(active_motion);
            }

            part_entities.push(part_commands.id());
        }
    });

    for object_effect in object.effects.iter() {
        let effect_transform = Transform::default()
            .with_translation(
                Vec3::new(
                    object_effect.position.x,
                    object_effect.position.z,
                    -object_effect.position.y,
                ) / 100.0,
            )
            .with_rotation(Quat::from_xyzw(
                object_effect.rotation.x,
                object_effect.rotation.z,
                -object_effect.rotation.y,
                object_effect.rotation.w,
            ))
            .with_scale(Vec3::new(
                object_effect.scale.x,
                object_effect.scale.z,
                object_effect.scale.y,
            ));

        if let Some(effect_path) = zsc.effects.get(object_effect.effect_id as usize) {
            if let Some(effect_entity) = spawn_effect(
                &vfs_resource.vfs,
                commands,
                asset_server,
                particle_materials,
                effect_mesh_materials,
                effect_path.into(),
                false,
                None,
            ) {
                if let Some(parent_part_entity) = object_effect
                    .parent
                    .and_then(|parent_part_index| part_entities.get(parent_part_index as usize))
                {
                    commands
                        .entity(*parent_part_entity)
                        .add_child(effect_entity);
                } else {
                    commands.entity(object_entity).add_child(effect_entity);
                }

                commands.entity(effect_entity).insert(effect_transform);

                if matches!(object_effect.effect_type, ZscEffectType::DayNight) {
                    commands.entity(effect_entity).insert(NightTimeEffect);
                }
            }
        }
    }

    object_entity
}

fn spawn_animated_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    object_materials: &mut Assets<ObjectMaterial>,
    stb_morph_object: &StbFile,
    object_instance: &IfoObject,
) -> Entity {
    let object_id = object_instance.object_id as usize;
    let mesh_path = stb_morph_object.get(object_id, 1);
    let motion_path = stb_morph_object.get(object_id, 2);
    let texture_path = stb_morph_object.get(object_id, 3);

    let alpha_enabled = stb_morph_object.get_int(object_id, 4) != 0;
    let two_sided = stb_morph_object.get_int(object_id, 5) != 0;
    let alpha_test_enabled = stb_morph_object.get_int(object_id, 6) != 0;
    let z_test_enabled = stb_morph_object.get_int(object_id, 7) != 0;
    let z_write_enabled = stb_morph_object.get_int(object_id, 8) != 0;

    // TODO: Animated object material blend op
    let _src_blend = stb_morph_object.get_int(object_id, 9);
    let _dst_blend = stb_morph_object.get_int(object_id, 10);
    let _blend_op = stb_morph_object.get_int(object_id, 11);

    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mesh = asset_server.load::<Mesh, _>(mesh_path);
    let material = object_materials.add(ObjectMaterial {
        base_texture: Some(
            asset_server.load(RgbTextureLoader::convert_path(Path::new(texture_path))),
        ),
        lightmap_texture: None,
        alpha_value: None,
        alpha_enabled,
        alpha_test: if alpha_test_enabled { Some(0.5) } else { None },
        two_sided,
        z_write_enabled,
        z_test_enabled,
        specular_enabled: false,
        blend: ObjectMaterialBlend::Normal,
        glow: None,
        skinned: false,
        lightmap_uv_offset: Vec2::new(0.0, 0.0),
        lightmap_uv_scale: 1.0,
    });

    // TODO: Animation object morph targets, blocked by lack of bevy morph targets
    commands
        .spawn((
            ZoneObject::AnimatedObject(ZoneObjectAnimatedObject {
                mesh_path: mesh_path.to_string(),
                motion_path: motion_path.to_string(),
                texture_path: texture_path.to_string(),
            }),
            mesh.clone(),
            material,
            object_transform,
            NotShadowCaster,
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            AsyncCollider {
                handle: mesh,
                shape: ComputedColliderShape::TriMesh,
            },
            CollisionGroups::new(COLLISION_GROUP_ZONE_OBJECT, COLLISION_FILTER_INSPECTABLE),
        ))
        .id()
}
