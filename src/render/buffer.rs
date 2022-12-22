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

    // TODO: in a real scenario, we are not supposed to call allocate memory for
    // each buffer separately, because these calls are limited to a relatively
    // small amount; instead we should create a custom allocator, that splits
    // up a single allocation among many differne objects by using the offset
    // parameters, that we've seen in many functions
    let buffer_memory = device.allocate_memory(&memory_info, None)?;

    // bind the memory to the vertex buffer
    device.bind_buffer_memory(
        buffer,
        buffer_memory,
        0, // no offset
    )?;
    Ok((buffer, buffer_memory))
}

pub unsafe fn copy_buffer(
    device: &Device,
    data: &AppData,
    source: vk::Buffer,
    destination: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<()> {
    // memory transfer operations are executed using commmand buffers
    // allocate temp command buffer
    let info = vk::CommandBufferAllocateInfo::builder()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(data.command_pool)
        .command_buffer_count(1);
    let command_buffer = device.allocate_command_buffers(&info)?[0];

    // record command buffer, we will only use it once
    let info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device.begin_command_buffer(command_buffer, &info)?;

    // define regions to copy, defined in BufferCopy-struct, which is source buffer offset,
    // destination buffer offset and size (we can't define vk::WHOLE_SIZE here..)
    let regions = vk::BufferCopy::builder().size(size);
    device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);

    // end recording
    device.end_command_buffer(command_buffer)?;

    // execute command buffer immediately
    let command_buffers = &[command_buffer];
    let info = vk::SubmitInfo::builder().command_buffers(command_buffers);

    // submit on graphics queue (which implicitly supports transfer operations)
    device.queue_submit(data.graphics_queue, &[info], vk::Fence::null())?;

    // we could use wait_for_fences here to schedule multiple transfers simultaneously
    // and wait for them to complete or just wait for the queue to idle
    device.queue_wait_idle(data.graphics_queue)?;

    device.free_command_buffers(data.command_pool, &[command_buffer]);

    Ok(())
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
