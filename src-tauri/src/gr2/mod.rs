use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
}

pub fn parse(_bytes: &[u8]) -> Result<Mesh, String> {
    Err("GR2-Parser folgt nach dem Research-Spike (siehe Roadmap Schritt 6)".into())
}
