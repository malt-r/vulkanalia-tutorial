#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

pub unsafe fn create_buffer(
    instance: &Instance,
    device: &Device,
    data: &AppData,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        // the vertex buffer will only be used by the graphics queue, so we don't
        // need to share it between queue families
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = device.create_buffer(&buffer_info, None)?;

    // allocate buffer memory
    let requirements = device.get_buffer_memory_requirements(buffer);

    let memory_info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            data,
            properties,
            requirements,
        )?);

    // allocate the buffer memory
    let buffer_memory = device.allocate_memory(&memory_info, None)?;

    // bind the memory to the vertex buffer
    device.bind_buffer_memory(
        buffer,
        buffer_memory,
        0, // no offset
    )?;
    Ok((buffer, buffer_memory))
}

// graphics cards offer more than one kind of memory with different allowed operations
// and performance characteristics; need to combine requirements for buffer and
// application requirements to get the right memory_type_index
unsafe fn get_memory_type_index(
    instance: &Instance,
    data: &AppData,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    // query about available type of memory
    // contains two arrays: memory_types and memory_heaps (distinct memory
    // ressources - i.e. VRAM or swap space in RAM)
    let memory = instance.get_physical_device_memory_properties(data.physical_device);

    // memory_type_bits of requirements specify the types of memory, which are
    // suitable
    // the vk::MemoryType entries in the memory_types array specify properties
    // (such as HOST_VISIBLE or HOST_COHERENT for memory which can be mapped to CPU
    // memory) - we need to also check for these properties
    //
    // summary: if there is a memory_type which is suitable for the buffer and
    //          also has all of the properties we need, we return it's index
    (0..memory.memory_type_count)
        .find(|i| {
            // TODO: read about this in vk docs
            let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
            let memory_type = memory.memory_types[*i as usize];
            suitable && memory_type.property_flags.contains(properties)
        })
        .ok_or_else(|| anyhow!("Failed to find suitable memory type."))
}
