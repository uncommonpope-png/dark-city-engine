use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MeshData {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
}

pub struct AssetManager {
    meshes: HashMap<String, MeshData>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
        }
    }

    pub fn register_mesh(&mut self, name: &str, data: MeshData) {
        self.meshes.insert(name.to_string(), data);
    }

    pub fn get_mesh(&self, name: &str) -> Option<&MeshData> {
        self.meshes.get(name)
    }

    pub fn has_mesh(&self, name: &str) -> bool {
        self.meshes.contains_key(name)
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }
}

pub fn create_cube_mesh() -> MeshData {
    // A simple cube with 24 vertices (4 per face, 6 faces)
    // Position (3) + Normal (3) + UV (2) = 8 floats per vertex
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let positions: &[[f32; 3]; 8] = &[
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];

    let faces: &[[u32; 6]; 6] = &[
        [0, 1, 2, 2, 3, 0], // -Z
        [5, 4, 7, 7, 6, 5], // +Z
        [4, 0, 3, 3, 7, 4], // -X
        [1, 5, 6, 6, 2, 1], // +X
        [3, 2, 6, 6, 7, 3], // +Y
        [4, 5, 1, 1, 0, 4], // -Y
    ];

    let normals: &[[f32; 3]; 6] = &[
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
    ];

    for (face_idx, face) in faces.iter().enumerate() {
        let normal = normals[face_idx];
        for &vert_idx in face.iter() {
            let pos = positions[vert_idx as usize];
            vertices.extend_from_slice(&pos);
            vertices.extend_from_slice(&normal);
            vertices.extend_from_slice(&[0.0, 0.0]); // UV placeholder
        }
        let base = (face_idx * 6) as u32;
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 3, base + 4, base + 5]);
    }

    MeshData { vertices, indices }
}

pub fn create_plane_mesh() -> MeshData {
    let mut vertices = Vec::new();
    let size = 5.0;
    let verts: &[[f32; 3]; 4] = &[
        [-size, 0.0, -size],
        [size, 0.0, -size],
        [size, 0.0, size],
        [-size, 0.0, size],
    ];
    let normal = [0.0, 1.0, 0.0];
    let uvs: &[[f32; 2]; 4] = &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    let idxs: &[u32; 6] = &[0, 1, 2, 0, 2, 3];
    for &i in idxs.iter() {
        let p = verts[i as usize];
        let uv = uvs[i as usize];
        vertices.extend_from_slice(&p);
        vertices.extend_from_slice(&normal);
        vertices.extend_from_slice(&uv);
    }

    MeshData {
        vertices,
        indices: idxs.to_vec(),
    }
}