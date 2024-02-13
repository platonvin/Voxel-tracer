extern crate vulkano;
extern crate winit;
extern crate exr;
extern crate fps_counter;
extern crate glam;

use std::{io::empty, ops::Sub, sync::Arc};

use fps_counter::FPSCounter;

use glam::{IVec3, Mat4};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferInfo, PrimaryCommandBufferAbstract};
use vulkano::device::{DeviceExtensions};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::swapchain::{self, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::{self, GpuFuture};
use vulkano::{DeviceSize, Validated, VulkanError};
use winit::{event::{Event, WindowEvent}};
use winit::event_loop::{ControlFlow, EventLoop};

mod renderer;
use renderer::*;
use renderer::loader::*;

const BLOCK_SIZE: usize = 16;
const CHUNK_SIZE: usize = 8;
const WORLD_SIZE: usize = 16;
// const VISIBLE_WORLD: usize = 8;

// #[derive(Debug)]

struct VoxelBlock {
    ///order is X -> Y -> Z. Each u8 is Material id in palette
    data: [u8; BLOCK_SIZE*BLOCK_SIZE*BLOCK_SIZE],
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
    data: [u8; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE],
}
/// All meshes shoult be reflected in world every frame
/// except for chunks, they are static and reflected on chunk loading (treat this as optimiztion, it could be done every frame but its pointless for now)
struct World {
    ///represents where chunks[0] starts
    current_origin: IVec3,
    ///order is X -> Y -> Z
    chunks: [VoxelChunk; 5*5*2],
    /// GPU-side buffer that stores world with all chunks within it
    united_blocks: Subbuffer<u8>,
}
impl VoxelBlock {
    //sets to Zero
    fn new() -> Self {
        VoxelBlock {data: [0; BLOCK_SIZE*BLOCK_SIZE*BLOCK_SIZE]}
    }
    //order is X -> Y -> Z
    fn get(&self, x: usize, y: usize, z: usize) -> u8 {
        let index = x + y*BLOCK_SIZE + z*BLOCK_SIZE*BLOCK_SIZE;
        self.data[index]
    }
    //order is X -> Y -> Z
    fn set(&mut self, x: usize, y: usize, z: usize, value: u8) {
        let index = x + y*BLOCK_SIZE + z*BLOCK_SIZE*BLOCK_SIZE;
        self.data[index] = value;
    }
}
impl VoxelChunk {
    //sets voxels to Zero, but requires Mesh
    fn new(mesh: Mesh) -> Self {
        VoxelChunk {
            mesh: mesh,
            data: [0; CHUNK_SIZE*CHUNK_SIZE*CHUNK_SIZE]
        }
    }
    //order is X -> Y -> Z
    fn get(&self, x: usize, y: usize, z: usize) -> u8 {
        let index = x + y*CHUNK_SIZE + z*CHUNK_SIZE*CHUNK_SIZE;
        self.data[index]
    }
    //order is X -> Y -> Z
    fn set(&mut self, x: usize, y: usize, z: usize, value: u8) {
        let index = x + y*CHUNK_SIZE + z*CHUNK_SIZE*CHUNK_SIZE;
        self.data[index] = value;
    }
}


pub fn load_map(){
    let scene = dot_vox::load("assets/scene.vox").unwrap();
    
    // scene.models
    // dot_vox::
}

fn main() {
    compile_shaders();
    
    let event_loop = EventLoop::new();
    let instance = create_instance(&event_loop);

    #[cfg(debug_assertions)] {let _debug_messanger = create_debug_messanger(instance.clone());}

    let window = create_window(&event_loop);
    let surface = create_surface(instance.clone(), window.clone());

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let (physical_device, queue_family_index) = select_physical_device(&instance, &surface, &device_extensions);
    let (device, mut queues) = create_device(physical_device.clone(), queue_family_index, device_extensions);
    //graphical and compute - required by spec to exist
    let queue = queues.next().unwrap();

    let (mut swapchain, swapchain_images) = create_swapchain(physical_device.clone(), device.clone(), surface.clone(), window.clone());

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    // let raygen_pos_mat_images = Image::new(memory_allocator.clone(), 
    // ImageCreateInfo {
    //     // image_type
    //     image_type: ImageType::Dim2d,
    //     format: vulkano::format::Format::R32G32B32A32_SFLOAT,
    //     extent: [swapchain.image_extent()[0], swapchain.image_extent()[1], 1],
    //     usage: ImageUsage::COLOR_ATTACHMENT,
    //     ..Default::default(
    // )}, 
    // AllocationCreateInfo {
    //     ..Default::default()
    // });
    // let raygen_norm_images = Image::new(memory_allocator.clone(), 
    // ImageCreateInfo {
    //     // image_type
    //     image_type: ImageType::Dim2d,
    //     format: vulkano::format::Format::R32G32B32A32_SFLOAT,
    //     extent: [swapchain.image_extent()[0], swapchain.image_extent()[1], 1],
    //     usage: ImageUsage::COLOR_ATTACHMENT,
    //     ..Default::default(
    // )}, 
    // AllocationCreateInfo {
    //     ..Default::default()
    // });

    let present_render_pass  = get_render_pass(device.clone(), swapchain.clone());
    let present_framebuffers = get_framebuffers(&swapchain_images, present_render_pass.clone());


    let vertex1 = MyVertex {
        position: [-0.5, -0.5],
    };
    let vertex2 = MyVertex {
        position: [0.0, 0.5],
    };
    let vertex3 = MyVertex {
        position: [0.5, -0.25],
    };
    let vertex_buffer = Buffer::from_iter(
        memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::TRANSFER_SRC,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        vec![vertex1, vertex2, vertex3],
    ).unwrap();

    let local_vertex_buffer = Buffer::new_slice::<MyVertex>(
        memory_allocator.clone(), 
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
            ..Default::default()
        },
        3 as DeviceSize
    ).unwrap();

    let command_buffer_allocator = StandardCommandBufferAllocator::new(
        device.clone(),
        StandardCommandBufferAllocatorCreateInfo::default(),
    );

    // Create a one-time command to copy between the buffers.
    let mut cbb = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    cbb.copy_buffer(CopyBufferInfo::buffers(
            vertex_buffer,
            local_vertex_buffer.clone(),
        ))
        .unwrap();
    let one_time_cb = cbb.build().unwrap();

    // Execute the copy command and wait for completion before proceeding.
    one_time_cb.execute(queue.clone())
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None /* timeout */)
        .unwrap();
    

    let vs = load_shader(device.clone(), "shaders/vert.spv");
    let fs = load_shader(device.clone(), "shaders/frag.spv");

    let mut viewport = Viewport {
        offset: [0.0, 0.0],
        extent: window.inner_size().into(),
        depth_range: 0.0..=1.0,
    };

    let pipeline = get_graphical_pipeline(
        device.clone(),
        vs.clone(),
        fs.clone(),
        present_render_pass.clone(),
        viewport.clone(),
    );

    let command_buffer_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());

    let mut command_buffers = get_command_buffers(
        &command_buffer_allocator,
        &queue,
        &pipeline,
        &present_framebuffers,
        &local_vertex_buffer,
    );

    let mut window_resized = false;
    let mut recreate_swapchain = false;

    let frames_in_flight = swapchain_images.len();
    let mut swapchain_fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];
    let mut previous_fence_i = 0;

    // let fps = fps_counter;
    let mut fps_counter = FPSCounter::new();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {event: WindowEvent::CloseRequested, ..} => {
            let fps = fps_counter.tick();
            println!("{}", fps);
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {event: WindowEvent::Resized(_), .. } => {
            window_resized = true;
        }
        Event::MainEventsCleared => {
            if window_resized || recreate_swapchain {
                recreate_swapchain = false;

                let new_dimensions = window.inner_size();

                let (new_swapchain, new_images) = swapchain
                    .recreate(SwapchainCreateInfo {
                        image_extent: new_dimensions.into(),
                        ..swapchain.create_info()
                    })
                    .expect("failed to recreate swapchain");

                swapchain = new_swapchain;
                let new_framebuffers = get_framebuffers(&new_images, present_render_pass.clone());

                if window_resized {
                    window_resized = false;

                    viewport.extent = new_dimensions.into();
                    let new_pipeline = get_graphical_pipeline(
                        device.clone(),
                        vs.clone(),
                        fs.clone(),
                        present_render_pass.clone(),
                        viewport.clone(),
                    );
                    command_buffers = get_command_buffers(
                        &command_buffer_allocator,
                        &queue,
                        &new_pipeline,
                        &new_framebuffers,
                        &local_vertex_buffer,
                    );
                }
            }

            let (image_i, suboptimal, acquire_future) =
                match swapchain::acquire_next_image(swapchain.clone(), None)
                    .map_err(Validated::unwrap)
                {
                    Ok(r) => r,
                    Err(VulkanError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(_e) => panic!("{}", "failed to acquire next image: {e}"),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            // wait for the fence related to this image to finish (normally this would be the oldest fence)
            if let Some(image_fence) = &swapchain_fences[image_i as usize] {
                image_fence.wait(None).unwrap();
            }

            let previous_future = match swapchain_fences[previous_fence_i as usize].clone() {
                // Create a NowFuture
                None => {
                    let mut now = sync::now(device.clone());
                    now.cleanup_finished();

                    now.boxed()
                }
                // Use the existing FenceSignalFuture
                Some(fence) => fence.boxed(),
            };

            let future = previous_future
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffers[image_i as usize].clone())
                .unwrap()
                .then_swapchain_present(
                    queue.clone(),
                    SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i),
                )
                .then_signal_fence_and_flush();

            swapchain_fences[image_i as usize] = match future.map_err(Validated::unwrap) {
                Ok(value) => Some(Arc::new(value)),
                Err(VulkanError::OutOfDate) => {
                    recreate_swapchain = true;
                    None
                }
                Err(e) => {
                    println!("failed to flush future: {e}");
                    None
                }
            };

            fps_counter.tick();

            
            previous_fence_i = image_i;
        }
        _ => (),
    });
}