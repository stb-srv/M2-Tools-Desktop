//! Rückgabe von `load_gr2_model` (siehe `src-tauri`, Granny-Loader).

use serde::Deserialize;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Gr2Mesh {
    pub name: String,
    pub is_rigid: bool,
    pub texture_name: Option<String>,
    /// Dekodierte Textur als PNG-Data-URL, `None` falls keine gefunden wurde.
    pub texture: Option<String>,
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct Gr2ModelInfo {
    pub name: String,
    pub bone_count: i64,
    pub meshes: Vec<Gr2Mesh>,
    pub skipped_meshes: i64,
}
