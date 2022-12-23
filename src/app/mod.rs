// logging
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::{ExtDebugUtilsExtension, KhrSurfaceExtension, KhrSwapchainExtension};
use vulkanalia::window as vk_window;

use winit::window::Window;

use crate::render::command_buffer;
use crate::render::command_pool;
use crate::render::device;
use crate::render::framebuffer;
use crate::render::instance;
use crate::render::pipeline;
use crate::render::render_pass;
use crate::render::swapchain;
use crate::render::synchronization;
use crate::render::validation;

use std::collections::VecDeque;
use std::{thread, time};

#[derive(Clone, Debug)]
pub struct App {
    entry: Entry,
    instance: Instance,
    data: AppData,
    pub device: Device,
    // current frame index for multiple frames in flight
    frame: usize,
    pub resized: bool,
    last_frame_end: time::Instant,
    samples: VecDeque<u128>,
    frame_counter: u32,
    sleep_in_render: bool,
}

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;
pub const FRAME_SAMPLE_COUNT: usize = 20;
//pub const SLEEP_IN_RENDER: bool = true;
pub const SLEEP_TIME_IN_MS: u32 = 16;

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

    pub image_ready_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,

    pub in_flight_fences: Vec<vk::Fence>,

    // used to keep track of which images are currently in flight
    // acquire_next_image_khr may return images out of order or MAX_FRAMES_IN_FLIGHT
    // could be higher than the number of swapchain images, so we could end up
    // rendering to a swapchain image, that is already in flight
    pub images_in_flight: Vec<vk::Fence>,

    // vertex input & buffer
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,

    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
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
        swapchain::create_swapchain_image_views(&device, &mut data)?;
        framebuffer::create_framebuffers(&device, &mut data)?;
        command_pool::create_command_pool(&instance, &device, &mut data)?;
        pipeline::create_vertex_buffer(&instance, &device, &mut data)?;
        pipeline::create_index_buffer(&instance, &device, &mut data)?;
        command_buffer::create_command_buffers(&device, &mut data)?;
        synchronization::create_sync_objects(&device, &mut data)?;

        let sleep = dotenv::var("SLEEP_IN_RENDER").unwrap();
        println!("Sleep: {0}", sleep);

        let sleep_bool = match sleep.as_str() {
            "0" => false,
            _ => true,
        };

        Ok(Self {
            entry,
            instance,
            data,
            device,
            frame: 0,
            resized: false,
            last_frame_end: time::Instant::now(),
            samples: VecDeque::with_capacity(FRAME_SAMPLE_COUNT),
            frame_counter: 0,
            sleep_in_render: sleep_bool,
        })
    }

    /// renders one frame
    pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
        self.device.wait_for_fences(
            &[self.data.in_flight_fences[self.frame]],
            true,
            u64::max_value(),
        )?;

        // Each of the actions required for rendering is executed by calling
        // a single function, which executes asynchronously -> requires synchronization
        //
        // Two types: Fences and Semaphores
        // - Fences: state can be queried from program to synchronize app itself
        //   with rendering
        // - Semaphores: state can't be queried from program, used to synchronize
        //   rendering internally
        let result = self.device.acquire_next_image_khr(
            self.data.swapchain,
            u64::max_value(),
            self.data.image_ready_semaphores[self.frame],
            vk::Fence::null(),
        );

        let image_index = match result {
            Ok((image_index, _)) => image_index as usize,
            Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                log::info!("Out of date khr, recreating swapchain");
                return self.recreate_swapchain(window);
            }
            Err(e) => return Err(anyhow!(e)),
        };

        // TODO: verstehen
        if !self.data.images_in_flight[image_index].is_null() {
            self.device.wait_for_fences(
                &[self.data.images_in_flight[image_index]],
                true,
                u64::max_value(),
            )?;
        }

        self.data.images_in_flight[image_index] = self.data.in_flight_fences[self.frame];

        let wait_semaphores = &[self.data.image_ready_semaphores[self.frame]];
        let command_buffers = &[self.data.command_buffers[image_index]];
        let signal_semaphores = &[self.data.render_finished_semaphores[self.frame]];
        let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores) // for which semaphore to wait
            .wait_dst_stage_mask(wait_stages) // in which stage(s) of the pipeline should we wait?
            // -> wait before the part of the pipeline, which
            // writes color to the color attachment
            .command_buffers(command_buffers) // which command_buffers should be used?
            .signal_semaphores(signal_semaphores); // which semaphores should be signaled on finish

        self.device
            .reset_fences(&[self.data.in_flight_fences[self.frame]])?;

        self.device.queue_submit(
            self.data.graphics_queue,
            &[submit_info],
            self.data.in_flight_fences[self.frame], // TODO: explain
        )?;

        let swapchains = &[self.data.swapchain];
        let image_indices = &[image_index as u32];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores) // what sem to wait for, before presentation happens
            // swapchains to present image to and index of image for each swapchain
            // -> almost always a single one
            .swapchains(swapchains)
            .image_indices(image_indices);

        // recreate swapchain, if it changed
        let result = self
            .device
            .queue_present_khr(self.data.present_queue, &present_info);

        let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
            || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

        if self.resized || changed {
            self.recreate_swapchain(window)?;
            self.resized = false;
        } else if let Err(e) = result {
            return Err(anyhow!(e));
        }

        self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

        //
        // --- fps counter ---
        //

        // TODO: refactor
        let now = time::Instant::now();
        let time = now - self.last_frame_end;

        self.last_frame_end = now;

        self.samples.push_front(time.as_nanos());
        if self.samples.len() >= FRAME_SAMPLE_COUNT {
            self.samples.pop_back();
        }

        self.frame_counter = self.frame_counter + 1;
        if self.frame_counter == FRAME_SAMPLE_COUNT as u32 {
            let avg: u128 = self.samples.iter().sum::<u128>() / self.samples.len() as u128 / 1000;
            let fps = 1_000_000 / avg;
            log::info!("Avg frame time: {} us, fps: {}", avg, fps);
            self.frame_counter = 0;
        }

        if self.sleep_in_render {
            thread::sleep(time::Duration::from_millis(SLEEP_TIME_IN_MS.into()));
        }
        Ok(())
    }

    pub unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        log::debug!("Recreating swapchain");

        // wait for device to become idle
        self.device.device_wait_idle()?;
        self.destroy_swapchain();

        swapchain::create_swapchain(window, &self.instance, &self.device, &mut self.data)?;
        swapchain::create_swapchain_image_views(&self.device, &mut self.data)?;
        render_pass::create_render_pass(&self.instance, &self.device, &mut self.data)?;
        pipeline::create_pipeline(&self.device, &mut self.data)?;
        framebuffer::create_framebuffers(&self.device, &mut self.data)?;
        command_buffer::create_command_buffers(&self.device, &mut self.data)?;
        self.data
            .images_in_flight
            .resize(self.data.swapchain_images.len(), vk::Fence::null());
        Ok(())
    }

    /// destroy the app
    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();

        self.destroy_swapchain();

        self.device.destroy_buffer(self.data.index_buffer, None);
        self.device.free_memory(self.data.index_buffer_memory, None);

        self.device.destroy_buffer(self.data.vertex_buffer, None);
        self.device
            .free_memory(self.data.vertex_buffer_memory, None);

        self.data
            .in_flight_fences
            .iter()
            .for_each(|f| self.device.destroy_fence(*f, None));

        self.data
            .render_finished_semaphores
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));

        self.data
            .image_ready_semaphores
            .iter()
            .for_each(|s| self.device.destroy_semaphore(*s, None));

        // destroying a command pool will free all ressources of the associated
        // command buffers
        self.device
            .destroy_command_pool(self.data.command_pool, None);

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

    unsafe fn destroy_swapchain(&self) {
        self.data
            .framebuffers
            .iter()
            .for_each(|f| self.device.destroy_framebuffer(*f, None));
        self.device
            .free_command_buffers(self.data.command_pool, &self.data.command_buffers);
        self.device.destroy_pipeline(self.data.pipeline, None);
        self.device
            .destroy_pipeline_layout(self.data.pipeline_layout, None);
        self.device.destroy_render_pass(self.data.render_pass, None);
        self.data
            .swapchain_image_views
            .iter()
            .for_each(|v| self.device.destroy_image_view(*v, None));
        self.device.destroy_swapchain_khr(self.data.swapchain, None);
    }
}
