extern crate dot_vox;

use std::sync::Arc;

use block_mesh::{ilattice::glam::{IVec3, Mat4}, ndshape::ConstShape3u32, GreedyQuadsBuffer, MergeVoxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG};
// use self::dot_vox::Voxel;
use vulkano::{buffer::Subbuffer, image::Image};

use crate::MyVertex;


const BLOCK_SIZE: usize = 18;
const CHUNK_SIZE: usize = 8;
const WORLD_SIZE: usize = 16;

#[derive(Clone, Copy)]
struct BlockID(u8);
#[derive(Clone, Copy, PartialEq, Eq)]
struct VoxelID(u8);
// #[derive(Debug)]
struct VoxelBlock {
    ///order is X -> Y -> Z. Each u8 is Material id in palette
    data: [VoxelID; BLOCK_SIZE*BLOCK_SIZE*BLOCK_SIZE],
}
struct Mesh {
    ///vertex data on gpu side
    vertices: Subbuffer<[MyVertex]>,
    ///index data on gpu side
    indices: Subbuffer<[u32]>,
    ///rotation, shift and scale it represents
    trans: Mat4,
}
struct VoxelChunk {
    ///order is X -> Y -> Z. Each u8 is VoxelBlock id in palette
    mesh: Mesh,
    /// is stored on CPU side ONLY because of physics 
    /// Also stored on GPU for rendering, Changing anything on CPU does not auto-change GPU side
    data: [BlockID; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE],
}
/// All meshes shoult be reflected in world every frame
/// except for chunks, they are static and reflected on chunk loading (treat this as optimiztion, it could be done every frame but its pointless for now)
struct World {
    ///represents where chunks[0] starts
    current_origin: IVec3,
    ///order is X -> Y -> Z
    /// currently just one chunk
    chunks: [VoxelChunk; 1*1*1],
    /// GPU-side 3d buffer (image) that stores world with all chunks within it
    /// copied every frame to somewhere and used to construct current voxelated scene world
    united_blocks: Arc<Image>,
    /// 3d array of 16x16x16 x 256 u8 that are material id's representing voxels
    block_palette: Arc<Image>,
    /// 1d array of materials 
    voxel_palette: Arc<Image>,
}
impl VoxelBlock {
    //sets to Zero
    fn new() -> Self {
        VoxelBlock {data: [VoxelID(0); BLOCK_SIZE*BLOCK_SIZE*BLOCK_SIZE]}
    }
    //order is X -> Y -> Z
    fn get(&self, x: usize, y: usize, z: usize) -> VoxelID {
        let index = x + y*BLOCK_SIZE + z*BLOCK_SIZE*BLOCK_SIZE;
        self.data[index]
    }
    //order is X -> Y -> Z
    fn set(&mut self, x: usize, y: usize, z: usize, value: VoxelID) {
        let index = x + y*BLOCK_SIZE + z*BLOCK_SIZE*BLOCK_SIZE;
        self.data[index] = value;
    }
}
impl VoxelChunk {
    ///order is X -> Y -> Z
    fn new(mesh: Mesh, data: [BlockID; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE]) -> Self {
        VoxelChunk {
            mesh: mesh,
            data: data
        }
    }
    fn get(&self, x: usize, y: usize, z: usize) -> BlockID {
        let index = x + y*CHUNK_SIZE + z*CHUNK_SIZE*CHUNK_SIZE;
        self.data[index]
    }
    fn set(&mut self, x: usize, y: usize, z: usize, value: BlockID) {
        let index = x + y*CHUNK_SIZE + z*CHUNK_SIZE*CHUNK_SIZE;
        self.data[index] = value;
    }
}

impl block_mesh::Voxel for VoxelID {
    fn get_visibility(&self) -> VoxelVisibility {
        if *self == VoxelID(0) {
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}
impl block_mesh::MergeVoxel for VoxelID {
    type MergeValue = Self;

    fn merge_value(&self) -> Self::MergeValue {
        *self
    }
}
//TODO MAKE DYNAMIC
type BlockShape = ConstShape3u32<18, 18, 18>;

pub fn load_map() -> Vec<MyVertex>{
    let scene = dot_vox::load("assets/scene.vox").unwrap();
    
    // let mut world: World;
    // let mut mesh: Mesh;
    let mut single_block = VoxelBlock::new(); //zeroed

    for voxel in &scene.models[0].voxels{
        let x = (voxel.x+1) as usize;
        let y = (voxel.y+1) as usize;
        let z = (voxel.z+1) as usize;
        single_block.data[x + BLOCK_SIZE*y + BLOCK_SIZE*BLOCK_SIZE*z] = VoxelID(voxel.i+1);
    }

    // let mut buffer = UnitQuadBuffer::new();
    let mut buffer = GreedyQuadsBuffer::new(single_block.data.len());
    block_mesh::greedy_quads(&single_block.data, &BlockShape {}, [0; 3], [17; 3], &RIGHT_HANDED_Y_UP_CONFIG.faces, &mut buffer);

    println!("{}", buffer.quads.num_quads());
    // println!("{}", buffer.groups.);
    // let triangles;
    let mut vertices = Vec::new();

    for face_i in 0..6{
        let face_dir = &RIGHT_HANDED_Y_UP_CONFIG.faces[face_i];
        let face_group = buffer.quads.groups[face_i].clone();

        let normals = face_dir.quad_mesh_normals();

        for quad in face_group{
            let positions = face_dir.quad_mesh_positions(&quad.into(), 1.0);
            // println!("{:#?}", positions[0]);
            vertices.push(MyVertex {position: positions[0  ], normal: normals[0  ], mat: 2});
            vertices.push(MyVertex {position: positions[1  ], normal: normals[1  ], mat: 2});
            vertices.push(MyVertex {position: positions[2  ], normal: normals[2  ], mat: 2});
            vertices.push(MyVertex {position: positions[1+0], normal: normals[1+0], mat: 2});
            vertices.push(MyVertex {position: positions[1+1], normal: normals[1+1], mat: 2});
            vertices.push(MyVertex {position: positions[1+2], normal: normals[1+2], mat: 2});
        }
    };

    return vertices;
    // return buffer;
}
