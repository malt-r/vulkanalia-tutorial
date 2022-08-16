// logging
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::{ExtDebugUtilsExtension, KhrSurfaceExtension, KhrSwapchainExtension};
use vulkanalia::window as vk_window;

use winit::window::Window;

use crate::render::command_pool;
use crate::render::device;
use crate::render::framebuffer;
use crate::render::instance;
use crate::render::pipeline;
use crate::render::render_pass;
use crate::render::swapchain;
use crate::render::validation;

#[derive(Clone, Debug)]
pub struct App {
    entry: Entry,
    instance: Instance,
    data: AppData,
    device: Device,
}

#[derive(Clone, Debug, Default)]
pub struct AppData {
    pub surface: vk::SurfaceKHR,
    pub messenger: vk::DebugUtilsMessengerEXT,
    // this will be implicitly destroyed, if the instance is destroyed,
    // so no further handling of this in App::destroy() required
    pub physical_device: vk::PhysicalDevice,

    // queues, which will be created along with logic device creation
    // queues are implicitly cleaned up, when the device is destroyed
    pub graphics_queue: vk::Queue,

    // the presentation queue also needs to be created with the logic
    // device
    pub present_queue: vk::Queue,

    // swapchain related data
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,

    // image views
    pub swapchain_image_views: Vec<vk::ImageView>,

    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,

    pub pipeline: vk::Pipeline,

    pub framebuffers: Vec<vk::Framebuffer>,

    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,

    pub image_ready_sem: vk::Semaphore,
    pub render_finished_sem: vk::Semaphore,
}

// TODO: expose own safe wrapper around vulkan calls, which asserts the calling
// of the correct invariants of the vulkan API functions
impl App {
    /// creates the app
    pub unsafe fn create(window: &Window) -> Result<Self> {
        // create library loader for the vulkan library (LIBRARY is a constant
        // path pointing to the Vulkan library)
        // this loads initial Vulkan commands from the library
        let loader = LibloadingLoader::new(LIBRARY)?;
        // load the entry point of the Vulkan library using the loader
        let entry = Entry::new(loader).map_err(|e| anyhow!("{}", e))?;
        // use the window and entry to create a vulkan instance
        let mut data = AppData::default();
        let instance = instance::create_instance(window, &entry, &mut data)?;

        // setup window surface
        data.surface = vk_window::create_surface(&instance, window)?;

        device::pick_physical_device(&instance, &mut data)?;
        let device = device::create_logical_device(&instance, &mut data)?;

        swapchain::create_swapchain(window, &instance, &device, &mut data)?;
        render_pass::create_render_pass(&instance, &device, &mut data)?;
        pipeline::create_pipeline(&device, &mut data)?;
        framebuffer::create_framebuffers(&device, &mut data)?;
        command_pool::create_command_pool(&instance, &device, &mut data)?;

        Ok(Self {
            entry,
            instance,
            data,
            device,
        })
    }

    /// renders one frame
    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        // Each of the actions required for rendering is executed by calling
        // a single function, which executes asynchronously -> requires synchronization
        //
        // Two types: Fences and Semaphores
        // - Fences: state can be queried from program to synchronize app itself
        //   with rendering
        // - Semaphores: state can't be queried from program, used to synchronize
        //   rendering internally
        // TODO: acquire image from swapchain
        let image_index = self
            .device
            .acquire_next_image_khr(
                self.data.swapchain,
                u64::max_value(),
                self.data.image_ready_sem,
                vk::Fence::null()
                )?.0 as usize;

        // TODO: execute command buffer
        let wait_semaphores = &[self.data.image_ready_sem];
        let command_buffers = &[self.data.command_buffers[image_index]];
        let signal_semaphores = &[self.data.render_finished_sem];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores) // for which semaphore to wait
            .wait_dst_stage_mask(wait_stages) // in which stage(s) of the pipeline should we wait?
                                              // -> wait before the part of the pipeline, which
                                              // writes color to the color attachment
            .command_buffers(command_buffers) // which command_buffers should be used?
            .signal_semaphores(signal_semaphores); // which semaphores should be signaled on finish

        self.device.queue_submit(self.data.graphics_queue, &[submit_info], vk::Fence::null())?;

        // TODO: return image to swapchain for presentation

        Ok(())
    }

    /// destroy the app
    pub unsafe fn destroy(&mut self) {
        self.device.destroy_semaphore(self.data.image_ready_sem, None);
        self.device.destroy_semaphore(self.data.render_finished_sem, None);

        // destroying a command pool will free all ressources of the associated
        // command buffers
        self.device
            .destroy_command_pool(self.data.command_pool, None);

        self.data
            .framebuffers
            .iter()
            .for_each(|f| self.device.destroy_framebuffer(*f, None));

        self.device.destroy_pipeline(self.data.pipeline, None);

        self.device
            .destroy_pipeline_layout(self.data.pipeline_layout, None);
        self.device.destroy_render_pass(self.data.render_pass, None);

        self.data
            .swapchain_image_views
            .iter()
            .for_each(|v| self.device.destroy_image_view(*v, None));

        self.device.destroy_swapchain_khr(self.data.swapchain, None);
        // None is for allocation callbacks
        self.device.destroy_device(None);

        // if validation is enabled, the debug messenger needs to be destroyed,
        // before the instance is destroyed
        if validation::ENABLED {
            self.instance
                .destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        self.instance.destroy_surface_khr(self.data.surface, None);
        // be explicit about it
        self.instance.destroy_instance(None);
    }
}
