use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;
use crate::render::queue;

// TODO: create as many command pools as there are images in flight (and command
// buffers) -> only one command buffer per command pool

pub unsafe fn create_command_pool(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    // command_pools are used to create command buffers, which will then be
    // submitted to a device queue -> each command_pool can only allocate
    // command_buffers, which are submitted to a single type of queue
    let indices = queue::QueueFamilyIndices::get(instance, data, data.physical_device)?;

    // get graphics queue
    let info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::empty()) // could use this to specify
        // hints to vulkan about usage of the command buffers, we want to create
        // from this pool
        .queue_family_index(indices.graphics);

    data.command_pool = device.create_command_pool(&info, None)?;
    Ok(())
}
