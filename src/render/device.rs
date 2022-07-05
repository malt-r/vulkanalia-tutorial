// logging
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};
use log::*;
use thiserror::Error;

use std::collections::HashSet;

use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk;

use super::queue::QueueFamilyIndices;
use super::validation;
use crate::app::AppData;

// The error macro of the thiserror-crate enables definition of custom
// error types in terms of structs or enums without all the boilerplate code
// which is required for implementing std::error::Error
//
// A custom error message can be defined with a format string, which uses members
// of data structure
#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);

const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

// we need to check, whether a given physical device
// is suitable to use for our needs
unsafe fn check_physical_device(
    instance: &Instance,
    data: &AppData,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    QueueFamilyIndices::get(instance, data, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;
    Ok(())
}

unsafe fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device, None)?
        .iter()
        .map(|e| e.extension_name)
        .collect::<HashSet<_>>();

    // TODO: print, which one is missing
    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        Err(anyhow!(SuitabilityError(
            "Missing required device extensions."
        )))
    }
}

pub unsafe fn create_logical_device(instance: &Instance, data: &mut AppData) -> Result<Device> {
    // specify queues to be created
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.presentation);

    // the queue priorities specify the prio of a queue for scheduling of
    // command execution
    //
    // From the tutorial:
    // "The current drivers will only allow you to create a small number of
    // queues for each queue family and you don't really need more than one."
    // That's because you can create all of the command buffers on multiple threads
    // and then submit them all at once from the main thread with a single low-overhead call"
    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    // enable device specific layers
    let layers = if validation::ENABLED {
        vec![validation::LAYER.as_ptr()]
    } else {
        vec![]
    };

    // specify used device features (queried for in check_physical_device)
    // TODO: nothing special required for now, specify later
    let features = vk::PhysicalDeviceFeatures::builder();

    // convert device_extension Strings to null terminated strings
    let extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    // create the logical device
    // TODO: what is this?
    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);
    let device = instance.create_device(data.physical_device, &info, None)?;

    // get handle to the graphics queue
    data.graphics_queue = device.get_device_queue(indices.graphics, 0);
    data.present_queue = device.get_device_queue(indices.presentation, 0);

    trace!("graphics queue family index: {}", indices.graphics);
    trace!("presentation queue family index: {}", indices.presentation);

    Ok(device)
}

pub unsafe fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()> {
    for device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(device);
        trace!("Checking physical device {}", properties.device_name);
        if let Err(error) = check_physical_device(instance, data, device) {
            warn!(
                "Skipping physical device ('{}'): {}",
                properties.device_name, error
            )
        } else {
            info!("Selecting physical device ('{}')", properties.device_name);
            data.physical_device = device;
            return Ok(());
        }
    }
    Err(anyhow!("Failed to select a physical device"))
}
