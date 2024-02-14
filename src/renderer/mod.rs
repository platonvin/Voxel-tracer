extern crate vulkano;
extern crate winit;
extern crate exr;
extern crate core;

pub mod loader;
pub mod world;
pub mod ogt_voxel_meshify;

use std::{ops::RangeInclusive, process::Command, sync::Arc};
use std::convert::TryInto;
use std::convert::TryFrom;

use vulkano::{buffer::{BufferContents, Subbuffer}, device::{DeviceCreateInfo, QueueCreateInfo}, format::Format, image::{ImageCreateInfo, ImageUsage}, instance::{debug::{DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo}, InstanceCreateInfo}, memory::allocator::{AllocationCreateInfo, MemoryAllocator, StandardMemoryAllocator}, pipeline::graphics::{depth_stencil::{CompareOp, DepthState, DepthStencilState}, rasterization::CullMode}, swapchain::{self, SwapchainCreateInfo}};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassBeginInfo, SubpassContents};
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{Device, DeviceExtensions, Queue, QueueFlags};
use vulkano::image::view::ImageView;
use vulkano::image::Image;
use vulkano::instance::Instance;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{Surface, Swapchain};
use winit::{event_loop::EventLoop, window::{Window, WindowBuilder}};


#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub struct MyVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
    #[format(R8_UINT)]
    pub mat: u8,
}

// #[derive(Debug)]
// pub struct Renderer{
//     library: vulkano::VulkanLibrary,
//     event_loop: EventLoop<()>,
//     instance: Instance,
//     window: Window,
//     surface: Surface,
//     physical_device: PhysicalDevice,
//     queue_graphics: Queue,

// }

pub fn select_physical_device(instance: &Arc<Instance>, surface: &Arc<Surface>, device_extensions: &DeviceExtensions,) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS) && q.queue_flags.contains(QueueFlags::COMPUTE) //100% exists (req by spec)
                        && p.surface_support(i as u32, surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        }).expect("no device available")
}

pub fn get_render_pass(device: Arc<Device>, swapchain: Arc<Swapchain>) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain.image_format(), // set the format the same as the swapchain
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth: {
                format: Format::D16_UNORM, // set the format the same as the swapchain
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth},
        },
    ).unwrap()
}

pub fn get_framebuffers(images: &[Arc<Image>], render_pass: Arc<RenderPass>, allocator: Arc<dyn MemoryAllocator>) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let depth_image = Image::new(allocator.clone(), ImageCreateInfo {
                image_type: vulkano::image::ImageType::Dim2d,
                format: Format::D16_UNORM,
                extent: image.extent(),
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                // initial_layout
                ..Default::default()
            }, AllocationCreateInfo {
                ..Default::default()
            }).unwrap();
            let depth_view = ImageView::new_default(depth_image).unwrap();

            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_view],
                    ..Default::default()
                },
            ).unwrap()
        }).collect::<Vec<_>>()
}

pub fn get_graphical_pipeline(device: Arc<Device>, vs: Arc<ShaderModule>, fs: Arc<ShaderModule>, render_pass: Arc<RenderPass>, viewport: Viewport) -> Arc<GraphicsPipeline> {
    let vs = vs.entry_point("main").unwrap();
    let fs = fs.entry_point("main").unwrap();

    let vertex_input_state = MyVertex::per_vertex()
        .definition(&vs.info().input_interface)
        .unwrap();

    let stages = [
        PipelineShaderStageCreateInfo::new(vs),
        PipelineShaderStageCreateInfo::new(fs),
    ];

    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .unwrap(),
    )
    .unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

    // let _aaa : Arc<Arc<Arc<Vec<u8>>>>;
    
    // let stage_refs: Vec<_> = stages.iter().collect();
    
    GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages: exr::prelude::SmallVec::from(stages).into_iter().collect(),
            vertex_input_state: Some(vertex_input_state),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state: Some(ViewportState {
                viewports: exr::prelude::SmallVec::from([viewport]).into_iter().collect(),
                ..Default::default()
            }),
            rasterization_state: Some(RasterizationState::default()),
            multisample_state: Some(MultisampleState::default()),
            color_blend_state: Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState::default(),
            )),
            subpass: Some(subpass.into()),
            depth_stencil_state: Some(DepthStencilState {
                // depth_bounds: Some(RangeInclusive::new(-1.0, 1.0)),
                depth: Some(DepthState {
                    write_enable: true,
                    compare_op: CompareOp::Less
                }),
                ..Default::default()
            }),
            // depth_stencil_state: Some(DepthStencilState::simple),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    ).unwrap()
}

pub fn get_command_buffers(
    command_buffer_allocator: &StandardCommandBufferAllocator,
    queue: &Arc<Queue>,
    pipeline: &Arc<GraphicsPipeline>,
    framebuffers: &[Arc<Framebuffer>],
    vertex_buffer: &Subbuffer<[MyVertex]>,
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    framebuffers.iter()
        .map(|framebuffer| {
            let mut builder = AutoCommandBufferBuilder::primary(
                command_buffer_allocator,
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )
            .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into()), Some(vulkano::format::ClearValue::Depth(1.0))],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::Inline,
                        ..Default::default()
                    },
                )
                .unwrap()
                .bind_pipeline_graphics(pipeline.clone())
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass(Default::default())
                .unwrap();

            builder.build().unwrap()
        }).collect()
}
// mod loader;
// use crate::renderer;

pub fn compile_shaders() {
    println!("shaders/v.vert -o vert.spv");
    Command::new("glslc").arg("shaders/v.vert").arg("-o").arg("shaders/vert.spv").output().unwrap();
    println!("shaders/v.frag -o frag.spv");
    Command::new("glslc").arg("shaders/v.frag").arg("-o").arg("shaders/frag.spv").output().unwrap();
}
pub fn create_instance(event_loop: &EventLoop<()>) -> Arc<Instance>{
    let library = vulkano::VulkanLibrary::new().expect("no local Vulkan library/DLL");

    let mut required_extensions = Surface::required_extensions(&event_loop);
    let mut enabled_layers: Vec<std::string::String>= vec![];

    // debug layers and extensions, might be setten to tell about perfomance and invalid usage
    #[cfg(debug_assertions)] {
        required_extensions.ext_debug_utils = true;
        enabled_layers.push("VK_LAYER_KHRONOS_validation".to_string());
    }
    let instance = Instance::new(
        library,
        InstanceCreateInfo {  
            enabled_extensions: required_extensions,
            enabled_layers: enabled_layers,
            ..Default::default()
        },
    ).expect("failed to create instance");

    return  instance;
}

pub fn create_debug_messanger(instance: Arc<Instance>) -> DebugUtilsMessenger{
unsafe {
        // let create_info = DebugUtilsMessengerCreateInfo { message_severity: (), message_type: (), user_callback: (), _ne: () };
    let dm = DebugUtilsMessenger::new(instance.clone(), DebugUtilsMessengerCreateInfo::user_callback(
        DebugUtilsMessengerCallback::new(|_severity, _msg_type, data| println!("{}", data.message)),
    )).unwrap();

    return dm;
}}

pub fn create_window(event_loop: &EventLoop<()>) -> Arc<Window>{
    Arc::new(WindowBuilder::new().build(&event_loop).unwrap())
}
pub fn create_surface(instance: Arc<Instance>, window: Arc<Window>) -> Arc<Surface>{
    Surface::from_window(instance, window).unwrap()
}
pub fn create_device(physical_device: Arc<PhysicalDevice>, family_index: u32, extensions: DeviceExtensions) -> (Arc<Device>, impl ExactSizeIterator<Item = Arc<Queue>>){
    let (device, queues) = Device::new(
        physical_device.clone(),
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index: family_index,
                ..Default::default()
            }],
            enabled_extensions: extensions, // new
            ..Default::default()
        },
    ).expect("failed to create device");

    return (device, queues);
}
pub fn create_swapchain(physical_device: Arc<PhysicalDevice>, device: Arc<Device>, surface: Arc<Surface>, window: Arc<Window>,) -> (Arc<Swapchain>, Vec<Arc<Image>>){
    let (swapchain, swapchain_images) = {
        let caps = physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let dimensions = window.inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = physical_device
            .surface_formats(&surface, Default::default())
            .unwrap()[0]
            .0;

        Swapchain::new(
            device.clone(),
            surface,
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count,
                image_format : image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                present_mode : swapchain::PresentMode::Fifo,
                ..Default::default()
            },
        ).unwrap()
    };
    return (swapchain, swapchain_images);
}