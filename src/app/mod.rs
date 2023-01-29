// logging
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::{ExtDebugUtilsExtension, KhrSurfaceExtension, KhrSwapchainExtension};
use vulkanalia::window as vk_window;

use winit::window::Window;

use nalgebra_glm as glm;

use crate::render::framebuffer;
use crate::render::instance;
use crate::render::pipeline;
use crate::render::render_pass;
use crate::render::swapchain;
use crate::render::synchronization;
use crate::render::validation;
use crate::render::{command_buffer, descriptor_set};
use crate::render::{command_pool, descriptor_pool};
use crate::render::{device, image};
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;

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
    count_fps: bool,
    start: time::Instant,
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
    pub descriptor_set_layout: vk::DescriptorSetLayout,
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

    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,

    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,
    pub texture_image_view: vk::ImageView,

    pub texture_sampler: vk::Sampler,

    // depth buffering is also image based
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
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
        descriptor_set::create_descriptor_set_layout(&device, &mut data)?;
        pipeline::create_pipeline(&device, &mut data)?;
        swapchain::create_swapchain_image_views(&device, &mut data)?;
        framebuffer::create_framebuffers(&device, &mut data)?;
        command_pool::create_command_pool(&instance, &device, &mut data)?;
        image::create_depth_objects(&instance, &device, &mut data)?;
        image::create_texture_image(&instance, &device, &mut data)?;
        image::create_texture_image_view(&device, &mut data)?;
        image::create_texture_sampler(&device, &mut data)?;
        pipeline::create_vertex_buffer(&instance, &device, &mut data)?;
        pipeline::create_index_buffer(&instance, &device, &mut data)?;
        pipeline::create_uniform_buffers(&instance, &device, &mut data)?;
        descriptor_pool::create_descriptor_pool(&device, &mut data)?;
        descriptor_set::create_descriptor_sets(&device, &mut data)?;
        command_buffer::create_command_buffers(&device, &mut data)?;
        synchronization::create_sync_objects(&device, &mut data)?;

        let sleep = dotenv::var("SLEEP_IN_RENDER").unwrap();
        println!("Sleep: {0}", sleep);

        let sleep_bool = match sleep.as_str() {
            "0" => false,
            _ => true,
        };

        let fps = dotenv::var("FPS_COUNTER").unwrap();
        println!("fps counter: {0}", fps);

        let fps_bool = match fps.as_str() {
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
            start: time::Instant::now(),
            samples: VecDeque::with_capacity(FRAME_SAMPLE_COUNT),
            frame_counter: 0,
            sleep_in_render: sleep_bool,
            count_fps: fps_bool,
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

        // it is important, that the uniform buffer is not updated, before
        // the fence is signaled; we need to be sure, that any previously
        // rendered frame to the acquired swapchain image is completed, before
        // savely updating the data in the uniform buffer
        self.update_uniform_buffer(image_index)?;

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
        if self.count_fps {
            let now = time::Instant::now();
            let time = now - self.last_frame_end;

            self.last_frame_end = now;

            self.samples.push_front(time.as_nanos());
            if self.samples.len() >= FRAME_SAMPLE_COUNT {
                self.samples.pop_back();
            }

            self.frame_counter = self.frame_counter + 1;
            if self.frame_counter == FRAME_SAMPLE_COUNT as u32 {
                let avg: u128 =
                    self.samples.iter().sum::<u128>() / self.samples.len() as u128 / 1000;
                let fps = 1_000_000 / avg;
                log::info!("Avg frame time: {} us, fps: {}", avg, fps);
                self.frame_counter = 0;
            }

            if self.sleep_in_render {
                thread::sleep(time::Duration::from_millis(SLEEP_TIME_IN_MS.into()));
            }
        }
        Ok(())
    }

    unsafe fn update_uniform_buffer(&self, image_index: usize) -> Result<()> {
        let time = self.start.elapsed().as_secs_f32();
        // define model view projection transformations in the ubo

        // model rotation will be around the z-axis using time
        // rotate 90 degrees per second
        let model = glm::rotate(
            &glm::identity(),                         // existing transformation
            time * glm::radians(&glm::vec1(90.0))[0], // rotation angle
            &glm::vec3(0.0, 0.0, 1.0),                // rotation axis
        );

        // look at geometry from aboce at 45 degree angle
        let view = glm::look_at(
            &glm::vec3(2.0, 2.0, 2.0), // position of the camera
            &glm::vec3(0.0, 0.0, 0.0), // where to look at
            &glm::vec3(0.0, 0.0, 1.0), // where is up
        );

        // we want to use the Vulkan depth range of 0.0 to 1.0 (and not the OpenGL
        // depth range of -1.0 to 1.0); zo = zero-to-one
        let mut proj = glm::perspective_rh_zo(
            self.data.swapchain_extent.width as f32 / self.data.swapchain_extent.height as f32, // aspect ratio
            glm::radians(&glm::vec1(45.0))[0], // fov
            0.1,                               // near plane
            10.0,                              // far plane
        );

        // GLM was originally designed for OpenGL, where the Y coord of the clip
        // coordinated is inverted; easiest way to compensate, is to flip sign on
        // the scaling factor of the Y axis in the projection matrix; if we don't
        // do this, the image will be rendered upside down
        proj[(1, 1)] *= -1.0;
        let ubo = descriptor_set::UniformBufferObject { model, view, proj };

        // update uniform buffer memory
        let memory = self.device.map_memory(
            self.data.uniform_buffers_memory[image_index],
            0,
            size_of::<descriptor_set::UniformBufferObject>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;

        memcpy(&ubo, memory.cast(), 1);

        // updating the ubo in this way is not the most efficient way to update
        // data in shaders frequently (that would be a push constant for small data)
        self.device
            .unmap_memory(self.data.uniform_buffers_memory[image_index]);

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
        pipeline::create_uniform_buffers(&self.instance, &self.device, &mut self.data)?;
        descriptor_pool::create_descriptor_pool(&self.device, &mut self.data)?;
        descriptor_set::create_descriptor_sets(&self.device, &mut self.data)?;
        command_buffer::create_command_buffers(&self.device, &mut self.data)?;
        self.data
            .images_in_flight
            .resize(self.data.swapchain_images.len(), vk::Fence::null());
        Ok(())
    }

    /// destroy the app
    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();

        self.device.destroy_sampler(self.data.texture_sampler, None);

        self.device
            .destroy_image_view(self.data.texture_image_view, None);
        self.device.destroy_image(self.data.texture_image, None);
        self.device
            .free_memory(self.data.texture_image_memory, None);

        self.destroy_swapchain();

        self.device
            .destroy_descriptor_set_layout(self.data.descriptor_set_layout, None);

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
        self.device
            .destroy_descriptor_pool(self.data.descriptor_pool, None);

        self.data
            .uniform_buffers
            .iter()
            .for_each(|b| self.device.destroy_buffer(*b, None));
        self.data
            .uniform_buffers_memory
            .iter()
            .for_each(|m| self.device.free_memory(*m, None));

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
