#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::KhrSurfaceExtension;

use super::device::SuitabilityError;
use crate::app::AppData;

#[derive(Debug, Copy, Clone)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub presentation: u32,
}

impl QueueFamilyIndices {
    /// gets queue familiy indices for specified vulkan instance and physical device;
    /// can't be constant, because these indices may vary from device to device
    pub unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties = instance.get_physical_device_queue_family_properties(physical_device);

        // look for the first queue family, which supports the GRAPHICS property
        let graphics_property = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        let mut present = None;

        // find the queue family, which supports presentation
        for (index, properties) in properties.iter().enumerate() {
            if instance.get_physical_device_surface_support_khr(
                physical_device,
                index as u32,
                data.surface,
            )? {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics_property, present) {
            Ok(Self {
                graphics,
                presentation: present,
            })
        } else {
            Err(anyhow!(SuitabilityError(
                "Missing required queue families."
            )))
        }
    }
}
