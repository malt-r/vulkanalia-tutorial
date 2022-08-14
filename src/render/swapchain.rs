#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::KhrSurfaceExtension;
use vulkanalia::vk::KhrSwapchainExtension;


use winit::window::Window;
use crate::app::AppData;
use crate::render::queue::QueueFamilyIndices;

#[derive(Clone, Debug)]
pub struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    pub unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
        ) -> Result<Self> {
        Ok(Self {
            capabilities: instance
                .get_physical_device_surface_capabilities_khr(
                    physical_device, data.surface)?,
            formats: instance
                .get_physical_device_surface_formats_khr(
                    physical_device, data.surface)?,
            present_modes: instance
                .get_physical_device_surface_present_modes_khr(
                    physical_device, data.surface)?,
        })
    }
}

fn get_swapchain_surface_format(
    formats: &[vk::SurfaceFormatKHR],
) -> vk::SurfaceFormatKHR {
    // each entry contains a color_space and a format member
    // - format: color channels and types (vk::Format::B8G8R8A8_SRGB -> BGR and
    //   alpha stored in 8 bit unsigned integer)
    // - color space: indicates, if e.g. the sRGB color space is supported or not
    //   with the SRGB_NONLINEAR flag
    //
    // we use sRGB, if it is available
    formats
        .iter()
        .cloned()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or_else(|| formats[0]) // use selected or first one, if sRGB is not available
}

fn get_swapchain_present_mode(
    present_modes: &[vk::PresentModeKHR]
) -> vk::PresentModeKHR {
    // check, if Mailbox mode is supported, otherwise select FIFO (guaranteed
    // to be supported)
    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

fn get_swapchain_extent(
    window: &Window,
    capabilities: vk::SurfaceCapabilitiesKHR,
    ) -> vk::Extent2D {
    // if current_extent is set to u32::max, then we need to set the extent
    // specifically to the inner_size of the window

    if capabilities.current_extent.width != u32::max_value() {
        capabilities.current_extent
    } else {
        let size = window.inner_size();

        // clamp v(alue) between min and max
        // first select minimum of max and v (the smallest one)
        // second select maximum of this value and the minimum
        let clamp = |min: u32, max: u32, v: u32| min.max(max.min(v));

        vk::Extent2D::builder()
            .width(clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                    size.width
                    ))
            .height(clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                    size.height
                    ))
            .build()
    }
}

pub unsafe fn create_swapchain(
    window: &Window,
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
    ) -> Result<()> {
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
    let support = SwapchainSupport::get(instance, data, data.physical_device)?;

    let surface_format = get_swapchain_surface_format(&support.formats);
    let present_mode = get_swapchain_present_mode(&support.present_modes);
    let extent = get_swapchain_extent(window, support.capabilities);

    // strictly sticking to the minimum image count would mean, that we
    // sometimes have to wait for the driver to complete internal operations
    // before we can acquire another image to render to
    let mut image_count = support.capabilities.min_image_count + 1;
    if support.capabilities.max_image_count != 0
        && image_count > support.capabilities.max_image_count {
            image_count = support.capabilities.max_image_count;
        }

    // define sharing mode for images, which are shared across multiple queue
    // families -> use concurrent mode, if graphics and presentation queue family
    // are not the same, otherwise use exclusive (concurrent needs at least 2 distinct
    // queue families)
    let mut queue_family_indices = vec![];
    let image_sharing_mode = if indices.graphics != indices.presentation {
        queue_family_indices.push(indices.graphics);
        queue_family_indices.push(indices.presentation);
        vk::SharingMode::CONCURRENT
    } else {
        vk::SharingMode::EXCLUSIVE
    };

    info!("Creating swapchain");

    // fill out the swapchain creation structure
    let info = vk::SwapchainCreateInfoKHR::builder()
        .surface(data.surface)
        // details of swapchain images
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1) // always 1, instead for stereoskopic 3D app..
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // COLOR_ATTACHMENT for
        // direct rendering, TRANSFER_DST for rendering to separate images
        .image_sharing_mode(image_sharing_mode)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(support.capabilities.current_transform) // could specify, that certain transform be applied to images in swapchain (90 deg rotate, horizontal flip)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE) // specifies, if alpha should be
        // used for blending with other windows in window system
        .present_mode(present_mode)
        .clipped(true) // don't care about pixels, which are obscured by other windows -> better performance
        .old_swapchain(vk::SwapchainKHR::null()); // if swapchain gets invalidated
        // (on window resize) we need to recreate it and pass the old one, but we
        // don't do that here

    data.swapchain = device.create_swapchain_khr(&info, None)?;
    data.swapchain_images = device.get_swapchain_images_khr(data.swapchain)?;
    data.swapchain_format = surface_format.format;
    data.swapchain_extent = extent;

    Ok(())
}
