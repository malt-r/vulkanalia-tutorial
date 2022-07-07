#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::KhrSurfaceExtension;




use winit::window::Window;
use crate::app::AppData;

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
