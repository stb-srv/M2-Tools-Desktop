export interface Gr2Mesh {
  name: string;
  vertices: number[];
  normals: number[];
  uvs: number[];
  indices: number[];
}

export interface Gr2ModelInfo {
  name: string;
  bone_count: number;
  meshes: Gr2Mesh[];
  skipped_skinned_meshes: number;
}
