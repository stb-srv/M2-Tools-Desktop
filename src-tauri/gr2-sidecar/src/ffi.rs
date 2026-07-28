#![allow(non_camel_case_types, non_snake_case, dead_code)]
// Hand-written FFI subset for the Granny2 SDK, covering exactly the surface
// GR2Loader.cpp (github.com/DerUranov1997/DuckySource) uses to extract rigid
// mesh geometry. Struct field order/types verified against vendor/granny/include/granny.h
// (GrannyTypeSizeCheck asserts confirm the layout for each struct below).

use std::os::raw::{c_char, c_void};

pub type granny_int32 = i32;
pub type granny_uint32 = u32;
pub type granny_real32 = f32;
pub type granny_triple = [granny_real32; 3];
pub type granny_quad = [granny_real32; 4];
pub type granny_matrix_4x4 = [[granny_real32; 4]; 4];

pub const GRANNY_END_MEMBER: i32 = 0;
pub const GRANNY_REAL32_MEMBER: i32 = 10;

pub const GRANNY_DIFFUSE_COLOR_TEXTURE: granny_int32 = 2;

#[repr(C)]
pub struct granny_file {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_vertex_data {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_tri_topology {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_art_tool_info {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_exporter_info {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_track_group {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_animation {
    _private: [u8; 0],
}
#[repr(C)]
pub struct granny_texture_image {
    _private: [u8; 0],
}

#[repr(C)]
pub struct granny_data_type_definition {
    pub Type: granny_int32,
    pub Name: *const c_char,
    pub ReferenceType: *mut granny_data_type_definition,
    pub ArrayWidth: granny_int32,
    pub Extra: [granny_int32; 3],
    pub Ignored_Ignored: usize,
}

#[repr(C)]
pub struct granny_variant {
    pub Type: *mut granny_data_type_definition,
    pub Object: *mut c_void,
}

#[repr(C)]
pub struct granny_transform {
    pub Flags: granny_uint32,
    pub Position: granny_triple,
    pub Orientation: granny_quad,
    pub ScaleShear: [granny_triple; 3],
}

#[repr(C)]
pub struct granny_bone {
    pub Name: *const c_char,
    pub ParentIndex: granny_int32,
    pub LocalTransform: granny_transform,
    pub InverseWorld4x4: granny_matrix_4x4,
    pub LODError: granny_real32,
    pub ExtendedData: granny_variant,
}

#[repr(C)]
pub struct granny_skeleton {
    pub Name: *const c_char,
    pub BoneCount: granny_int32,
    pub Bones: *mut granny_bone,
    pub LODType: granny_int32,
    pub ExtendedData: granny_variant,
}

#[repr(C)]
pub struct granny_model_mesh_binding {
    pub Mesh: *mut granny_mesh,
}

#[repr(C)]
pub struct granny_model {
    pub Name: *const c_char,
    pub Skeleton: *mut granny_skeleton,
    pub InitialPlacement: granny_transform,
    pub MeshBindingCount: granny_int32,
    pub MeshBindings: *mut granny_model_mesh_binding,
    pub ExtendedData: granny_variant,
}

#[repr(C)]
pub struct granny_material_map {
    pub Usage: *const c_char,
    pub Material: *mut granny_material,
}

#[repr(C)]
pub struct granny_material {
    pub Name: *const c_char,
    pub MapCount: granny_int32,
    pub Maps: *mut granny_material_map,
    pub Texture: *mut granny_texture,
    pub ExtendedData: granny_variant,
}

#[repr(C)]
pub struct granny_pixel_layout {
    pub BytesPerPixel: granny_int32,
    pub ShiftForComponent: [granny_int32; 4],
    pub BitsForComponent: [granny_int32; 4],
}

#[repr(C)]
pub struct granny_texture {
    pub FromFileName: *const c_char,
    pub TextureType: granny_int32,
    pub Width: granny_int32,
    pub Height: granny_int32,
    pub Encoding: granny_int32,
    pub SubFormat: granny_int32,
    pub Layout: granny_pixel_layout,
    pub ImageCount: granny_int32,
    pub Images: *mut granny_texture_image,
    pub ExtendedData: granny_variant,
}

#[repr(C)]
pub struct granny_material_binding {
    pub Material: *mut granny_material,
}

#[repr(C)]
pub struct granny_bone_binding {
    pub BoneName: *const c_char,
    pub OBBMin: granny_triple,
    pub OBBMax: granny_triple,
    pub TriangleCount: granny_int32,
    pub TriangleIndices: *mut granny_int32,
}

#[repr(C)]
pub struct granny_morph_target {
    pub ScalarName: *const c_char,
    pub VertexData: *mut granny_vertex_data,
    pub DataIsDeltas: granny_int32,
}

#[repr(C)]
pub struct granny_mesh {
    pub Name: *const c_char,
    pub PrimaryVertexData: *mut granny_vertex_data,
    pub MorphTargetCount: granny_int32,
    pub MorphTargets: *mut granny_morph_target,
    pub PrimaryTopology: *mut granny_tri_topology,
    pub MaterialBindingCount: granny_int32,
    pub MaterialBindings: *mut granny_material_binding,
    pub BoneBindingCount: granny_int32,
    pub BoneBindings: *mut granny_bone_binding,
    pub ExtendedData: granny_variant,
}

#[repr(C)]
pub struct granny_tri_material_group {
    pub MaterialIndex: granny_int32,
    pub TriFirst: granny_int32,
    pub TriCount: granny_int32,
}

#[repr(C)]
pub struct granny_file_info {
    pub ArtToolInfo: *mut granny_art_tool_info,
    pub ExporterInfo: *mut granny_exporter_info,
    pub FromFileName: *const c_char,
    pub TextureCount: granny_int32,
    pub Textures: *mut *mut granny_texture,
    pub MaterialCount: granny_int32,
    pub Materials: *mut *mut granny_material,
    pub SkeletonCount: granny_int32,
    pub Skeletons: *mut *mut granny_skeleton,
    pub VertexDataCount: granny_int32,
    pub VertexDatas: *mut *mut granny_vertex_data,
    pub TriTopologyCount: granny_int32,
    pub TriTopologies: *mut *mut granny_tri_topology,
    pub MeshCount: granny_int32,
    pub Meshes: *mut *mut granny_mesh,
    pub ModelCount: granny_int32,
    pub Models: *mut *mut granny_model,
    pub TrackGroupCount: granny_int32,
    pub TrackGroups: *mut *mut granny_track_group,
    pub AnimationCount: granny_int32,
    pub Animations: *mut *mut granny_animation,
    pub ExtendedData: granny_variant,
}

pub type granny_log_function = extern "C" fn(
    Type: granny_int32,
    Origin: granny_int32,
    File: *const c_char,
    Line: granny_int32,
    Message: *const c_char,
    UserData: *mut c_void,
);

#[repr(C)]
pub struct granny_log_callback {
    pub Function: Option<granny_log_function>,
    pub UserData: *mut c_void,
}

// The sidecar resolves these dynamically (GetProcAddress against the decorated
// stdcall names, e.g. "_GrannyReadEntireFile@4") instead of static-linking,
// since it loads whichever granny2.dll ships with the user's own Metin2 client.
pub type FnReadEntireFile = extern "system" fn(FileName: *const c_char) -> *mut granny_file;
pub type FnGetFileInfo = extern "system" fn(File: *mut granny_file) -> *mut granny_file_info;
pub type FnFreeFile = extern "system" fn(File: *mut granny_file);
pub type FnSetLogCallback = extern "system" fn(LogCallback: *const granny_log_callback);
pub type FnGetMeshVertexCount = extern "system" fn(Mesh: *const granny_mesh) -> granny_int32;
pub type FnCopyMeshVertices = extern "system" fn(
    Mesh: *const granny_mesh,
    VertexType: *const granny_data_type_definition,
    DestVertices: *mut c_void,
);
pub type FnGetMeshIndexCount = extern "system" fn(Mesh: *const granny_mesh) -> granny_int32;
pub type FnCopyMeshIndices = extern "system" fn(
    Mesh: *const granny_mesh,
    BytesPerIndex: granny_int32,
    DestIndices: *mut c_void,
);
pub type FnMeshIsRigid = extern "system" fn(Mesh: *const granny_mesh) -> bool;
