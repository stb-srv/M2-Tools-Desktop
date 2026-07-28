export interface Gr2Mesh {
  name: string;
  is_rigid: boolean;
  vertices: number[];
  normals: number[];
  uvs: number[];
  indices: number[];
}

export interface Gr2ModelInfo {
  name: string;
  bone_count: number;
  meshes: Gr2Mesh[];
  skipped_meshes: number;
}
