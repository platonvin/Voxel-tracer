extern crate dot_vox;

use std::{boxed, convert::TryInto, sync::Arc};

use block_mesh::{ilattice::glam::{IVec3, Mat4, Vec3, Vec4}, ndshape::ConstShape3u32, GreedyQuadsBuffer, MergeStrategy, MergeVoxel, UnitQuadBuffer, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG};
// use self::dot_vox::Voxel;
use vulkano::{buffer::Subbuffer, image::Image};

use crate::MyVertex;


const BLOCK_SIZE: usize = 16;
const CHUNK_SIZE: usize = 8;
const WORLD_SIZE: usize = 16;

#[derive(Clone, Copy)]
pub struct BlockID(u8);
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VoxelID(u8);
// #[derive(Debug)]
pub struct VoxelBlock {
    ///order is X -> Y -> Z. Each u8 is Material id in palette
    data: Box<[VoxelID; BLOCK_SIZE*BLOCK_SIZE*BLOCK_SIZE]>,
}
#[derive(Clone)]
pub struct MeshCPU {
    ///vertex data on gpu side
    pub vertices: Vec<MyVertex>,
    ///index data on gpu side
    // pub indices: Subbuffer<[u32]>,
    ///rotation, shift and scale it represents
    pub trans: Mat4,
}
pub struct VoxelChunk {
    ///order is X -> Y -> Z. Each u8 is VoxelBlock id in palette
    pub mesh: MeshCPU,
    /// is stored on CPU side ONLY because of physics 
    /// Also stored on GPU for rendering, Changing anything on CPU does not auto-change GPU side
    data: Box<[BlockID; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE]>,
}
#[derive(Clone, Copy)]
pub struct Material {
    color: Vec4,
    emmitance: f32,
    roughness: f32,
}
/// All meshes shoult be reflected in world every frame
/// except for chunks, they are static and reflected on chunk loading (treat this as optimiztion, it could be done every frame but its pointless for now)
pub struct World {
    ///represents where chunks[0] starts
    // pub current_origin: IVec3,
    ///order is X -> Y -> Z
    /// currently just one chunk
    pub block_palette: Box<[VoxelID; 16*16*16*256]>, // 
    pub voxel_palette: Box<[Material; 256]>,// 256,
    pub chunks: Box<[VoxelChunk; 1*1*1]>,
    // GPU-side 3d buffer (image) that stores world with all chunks within it
    // copied every frame to somewhere and used to conpub struct current voxelated scene world
    // pub united_blocks_image: Arc<Image>,
    // 3d array of 16x16x16 x 256 u8 that are material id's representing voxels
    // pub block_palette_image: Arc<Image>,
    // 1d array of materials 
    // pub voxel_palette_image: Arc<Image>,
}
impl VoxelBlock {
    //sets to Zero
    fn new() -> Self {
        VoxelBlock {data: Box::new([VoxelID(0); 16*16*16])}
    }
//     //order is X -> Y -> Z
//     fn get(&self, x: usize, y: usize, z: usize) -> VoxelID {
//         let index = x + y*BLOCK_SIZE + z*BLOCK_SIZE*BLOCK_SIZE;
//         self.data[index]
//     }
//     //order is X -> Y -> Z
//     fn set(&mut self, x: usize, y: usize, z: usize, value: VoxelID) {
//         let index = x + y*BLOCK_SIZE + z*BLOCK_SIZE*BLOCK_SIZE;
//         self.data[index] = value;
//     }
}
impl VoxelChunk {
    ///order is X -> Y -> Z
    fn new(mesh: MeshCPU, data: Box<[BlockID; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE]>) -> Self {
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
        VoxelID(1)
        // *self
    }
}
//TODO MAKE DYNAMIC
type BlockShape = ConstShape3u32<18, 18, 18>;

impl World {
    pub fn new() -> World{
        World {
            block_palette: Box::new([VoxelID(0); 16*16*16*256]),
            voxel_palette: Box::new([
                Material {
                    color: Vec4::new(0.0,0.0,0.0,0.0), 
                    emmitance: 0.0, 
                    roughness: 0.0};
                256]),
            chunks: Box::new([VoxelChunk {
                mesh: MeshCPU {
                    vertices: Vec::new(),
                    trans: Mat4::IDENTITY,
                },
                data: Box::new([BlockID(0); CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE]),
            }; 1])
        }
    }

    /// order is X -> Y -> Z
    /// 
    fn set_block_in_palette(&mut self, block: VoxelBlock, id: usize) {
        for x in 0..16{
        for y in 0..16{
        for z in 0..16{
            let real_z = z + id*16;
            self.block_palette[x + 16*y + 16*16*real_z] = block.data[x + 16*y + 16*16*z];
        }
        }
        }
    }

    pub fn load_map(&mut self){
        // println!("lmao");
        let scene = dot_vox::load("assets/scene.vox").unwrap();
        // scene.materials
        
        let mut block_id = 1;
        for model in &scene.models {
            let mut current_block = VoxelBlock::new(); //zeroed
            //used as temporary storage for meshification
            let mut temp_block = [VoxelID(0); 18*18*18];

            for voxel in &model.voxels{
                let x = voxel.x as usize;
                let y = voxel.y as usize;
                let z = voxel.z as usize;
                current_block.data[ x + BLOCK_SIZE*y + BLOCK_SIZE*BLOCK_SIZE*z   ] = VoxelID(voxel.i+1);
                   temp_block     [(x+1) +     18*(y+1) +             18*18*(z+1)] = VoxelID(voxel.i+1);
            }
            
            // let mut current_buffer = UnitQuadBuffer::new();
            let mut current_buffer = GreedyQuadsBuffer::new(temp_block.len());
            block_mesh::greedy_quads(&temp_block, &BlockShape{}, [0; 3], [17; 3], &RIGHT_HANDED_Y_UP_CONFIG.faces, &mut current_buffer);
            // block_mesh::visible_block_faces(&temp_block, &BlockShape{}, [0; 3], [17; 3], &RIGHT_HANDED_Y_UP_CONFIG.faces, &mut current_buffer);
            
            // println!("{}", current_buffer.quads.num_quads());
            
            for face_i in 0..6{
                let face_dir = &RIGHT_HANDED_Y_UP_CONFIG.faces[face_i];
                let face_group = &current_buffer.quads.groups[face_i];
                // let face_group = &current_buffer.groups[face_i];
                
                let normals = face_dir.quad_mesh_normals();
                
                for &quad in face_group{
                    // let positions = face_dir.quad_mesh_positions(&quad.into(), 1.0);
                    let positions = face_dir.quad_mesh_positions(&quad, 1.0);
                    // let corners = face_dir.quad_corners(&quad.into());
                    let corners = face_dir.quad_corners(&quad);

                    let mut mats = [VoxelID(0); 4];
                    for i in 0..4{
                        let x = (corners[i].x) as usize;
                        let y = (corners[i].y) as usize;
                        let z = (corners[i].z) as usize;
                        mats[i] = temp_block[x + 18*y + 18*18*z];
                     // print!("{} ", corners[i]);
                    }
                    // println!();
                    
                    self.chunks[0].mesh.vertices.push(MyVertex {position: positions[0  ], normal: normals[0  ], mat: mats[0  ].0});
                    self.chunks[0].mesh.vertices.push(MyVertex {position: positions[1  ], normal: normals[1  ], mat: mats[1  ].0});
                    self.chunks[0].mesh.vertices.push(MyVertex {position: positions[2  ], normal: normals[2  ], mat: mats[2  ].0});
                    self.chunks[0].mesh.vertices.push(MyVertex {position: positions[1+0], normal: normals[1+0], mat: mats[1+0].0});
                    self.chunks[0].mesh.vertices.push(MyVertex {position: positions[1+1], normal: normals[1+1], mat: mats[1+1].0});
                    self.chunks[0].mesh.vertices.push(MyVertex {position: positions[1+2], normal: normals[1+2], mat: mats[1+2].0});
                }
            };
            self.set_block_in_palette(current_block, block_id);
            block_id+=1;
        };
    }
}
