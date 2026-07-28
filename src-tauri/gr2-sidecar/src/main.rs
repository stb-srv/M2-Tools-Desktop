mod ffi;

use ffi::*;
use serde::Serialize;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

#[derive(Debug, Clone, Serialize, Default)]
struct Mesh {
    name: String,
    vertices: Vec<f32>,
    normals: Vec<f32>,
    uvs: Vec<f32>,
    indices: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct ModelInfo {
    name: String,
    bone_count: i32,
    meshes: Vec<Mesh>,
    skipped_skinned_meshes: usize,
}

#[derive(Serialize)]
struct Output {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<ModelInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
}

struct GrannyApi {
    read_entire_file: FnReadEntireFile,
    get_file_info: FnGetFileInfo,
    free_file: FnFreeFile,
    get_mesh_vertex_count: FnGetMeshVertexCount,
    copy_mesh_vertices: FnCopyMeshVertices,
    get_mesh_index_count: FnGetMeshIndexCount,
    copy_mesh_indices: FnCopyMeshIndices,
    mesh_is_rigid: FnMeshIsRigid,
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn resolve<T: Copy>(module: *mut c_void, decorated_name: &str) -> Result<T, String> {
    let c_name = CString::new(decorated_name).unwrap();
    let addr = GetProcAddress(module, c_name.as_ptr());
    if addr.is_null() {
        return Err(format!("Symbol nicht gefunden: {decorated_name}"));
    }
    Ok(std::mem::transmute_copy::<*mut c_void, T>(&addr))
}

unsafe fn load_granny(dll_path: &str) -> Result<GrannyApi, String> {
    let wide_path = to_wide(dll_path);
    let module = LoadLibraryW(wide_path.as_ptr());
    if module.is_null() {
        return Err(format!(
            "granny2.dll konnte nicht geladen werden (32-Bit-Prozess?): {dll_path}"
        ));
    }

    Ok(GrannyApi {
        read_entire_file: resolve(module, "_GrannyReadEntireFile@4")?,
        get_file_info: resolve(module, "_GrannyGetFileInfo@4")?,
        free_file: resolve(module, "_GrannyFreeFile@4")?,
        get_mesh_vertex_count: resolve(module, "_GrannyGetMeshVertexCount@4")?,
        copy_mesh_vertices: resolve(module, "_GrannyCopyMeshVertices@12")?,
        get_mesh_index_count: resolve(module, "_GrannyGetMeshIndexCount@4")?,
        copy_mesh_indices: resolve(module, "_GrannyCopyMeshIndices@12")?,
        mesh_is_rigid: resolve(module, "_GrannyMeshIsRigid@4")?,
    })
}

unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

fn rigid_vertex_type() -> [granny_data_type_definition; 4] {
    let entry = |name: &'static CStr, width: i32| granny_data_type_definition {
        Type: GRANNY_REAL32_MEMBER,
        Name: name.as_ptr(),
        ReferenceType: ptr::null_mut(),
        ArrayWidth: width,
        Extra: [0; 3],
        Ignored_Ignored: 0,
    };
    [
        entry(c"Position", 3),
        entry(c"Normal", 3),
        entry(c"TextureCoordinates0", 2),
        granny_data_type_definition {
            Type: GRANNY_END_MEMBER,
            Name: ptr::null(),
            ReferenceType: ptr::null_mut(),
            ArrayWidth: 0,
            Extra: [0; 3],
            Ignored_Ignored: 0,
        },
    ]
}

unsafe fn extract_mesh(api: &GrannyApi, mesh_ptr: *mut granny_mesh) -> Option<Mesh> {
    if mesh_ptr.is_null() || !(api.mesh_is_rigid)(mesh_ptr) {
        return None;
    }
    let name = cstr_to_string((*mesh_ptr).Name);

    let vertex_count = (api.get_mesh_vertex_count)(mesh_ptr) as usize;
    let vertex_type = rigid_vertex_type();
    let mut raw_vertices = vec![0f32; vertex_count * 8];
    (api.copy_mesh_vertices)(
        mesh_ptr,
        vertex_type.as_ptr(),
        raw_vertices.as_mut_ptr() as *mut c_void,
    );

    let mut vertices = Vec::with_capacity(vertex_count * 3);
    let mut normals = Vec::with_capacity(vertex_count * 3);
    let mut uvs = Vec::with_capacity(vertex_count * 2);
    for v in raw_vertices.chunks_exact(8) {
        vertices.extend_from_slice(&v[0..3]);
        normals.extend_from_slice(&v[3..6]);
        uvs.extend_from_slice(&v[6..8]);
    }

    let index_count = (api.get_mesh_index_count)(mesh_ptr) as usize;
    let mut raw_indices = vec![0u16; index_count];
    (api.copy_mesh_indices)(mesh_ptr, 2, raw_indices.as_mut_ptr() as *mut c_void);
    let indices: Vec<u32> = raw_indices.into_iter().map(|i| i as u32).collect();

    Some(Mesh {
        name,
        vertices,
        normals,
        uvs,
        indices,
    })
}

unsafe fn parse_model(api: &GrannyApi, path: &str) -> Result<ModelInfo, String> {
    let c_path = CString::new(path).map_err(|e| e.to_string())?;
    let file = (api.read_entire_file)(c_path.as_ptr());
    if file.is_null() {
        return Err(format!("GR2-Datei konnte nicht gelesen werden: {path}"));
    }

    let file_info = (api.get_file_info)(file);
    if file_info.is_null() {
        (api.free_file)(file);
        return Err("Keine Modellinformationen in der GR2-Datei gefunden".into());
    }

    let info = &*file_info;
    if info.ModelCount == 0 || info.Models.is_null() {
        (api.free_file)(file);
        return Err("Keine Modelle in der GR2-Datei enthalten".into());
    }

    let model_ptr = *info.Models;
    let model = &*model_ptr;
    let name = cstr_to_string(model.Name);
    let bone_count = if model.Skeleton.is_null() {
        0
    } else {
        (*model.Skeleton).BoneCount
    };

    let mut meshes = Vec::new();
    let mut skipped = 0usize;
    if !model.MeshBindings.is_null() {
        let bindings =
            std::slice::from_raw_parts(model.MeshBindings, model.MeshBindingCount.max(0) as usize);
        for binding in bindings {
            match extract_mesh(api, binding.Mesh) {
                Some(mesh) => meshes.push(mesh),
                None => skipped += 1,
            }
        }
    }

    (api.free_file)(file);

    Ok(ModelInfo {
        name,
        bone_count,
        meshes,
        skipped_skinned_meshes: skipped,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let output = if args.len() != 3 {
        Output {
            ok: false,
            data: None,
            error: Some("Usage: gr2-sidecar <granny2.dll path> <.gr2 file path>".into()),
        }
    } else {
        let dll_path = &args[1];
        let gr2_path = &args[2];
        unsafe {
            match load_granny(dll_path).and_then(|api| parse_model(&api, gr2_path)) {
                Ok(data) => Output {
                    ok: true,
                    data: Some(data),
                    error: None,
                },
                Err(error) => Output {
                    ok: false,
                    data: None,
                    error: Some(error),
                },
            }
        }
    };

    println!("{}", serde_json::to_string(&output).unwrap());
}
