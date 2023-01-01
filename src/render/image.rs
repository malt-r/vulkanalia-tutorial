#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use std::fs::File;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use crate::{
    app::AppData,
    render::buffer::{self, get_memory_type_index},
};

use super::command_buffer;

pub unsafe fn create_texture_image(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let image = File::open("resources/texture.png")?;

    let decoder = png::Decoder::new(image);
    let mut reader = decoder.read_info()?;
    // 4 bytes per pixel
    let mut pixels = vec![0; reader.info().raw_bytes()];
    reader.next_frame(&mut pixels)?;

    let size = reader.info().raw_bytes() as u64;
    let (width, height) = reader.info().size();
    log::debug!("Raw bytes size: {}", size);
    log::debug!("width: {}, height: {}", width, height);
    log::debug!(
        "width * height * 4: {}",
        width as u64 * height as u64 * 4 as u64
    );

    // the following lines expect the image to have an Alpha channel; the png
    // crate does not support converting from RGB to RGBA, so make sure, that
    // the image has an alpha channel

    // stage image data in host visible memory
    let (staging_buffer, staging_buffer_memory) = buffer::create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // copy pixel data to staging buffer memory
    let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;

    memcpy(pixels.as_ptr(), memory.cast(), pixels.len());

    device.unmap_memory(staging_buffer_memory);

    let (texture_image, texture_image_memory) = create_image(
        instance,
        device,
        data,
        width,
        height,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    data.texture_image = texture_image;
    data.texture_image_memory = texture_image_memory;

    Ok(())
}

pub unsafe fn create_image(
    instance: &Instance,
    device: &Device,
    data: &AppData,
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    let info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .samples(vk::SampleCountFlags::_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let image = device.create_image(&info, None)?;
    let requirements = device.get_image_memory_requirements(image);

    let info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(get_memory_type_index(
            instance,
            data,
            properties,
            requirements,
        )?);

    let image_memory = device.allocate_memory(&info, None)?;

    device.bind_image_memory(image, image_memory, 0)?;

    Ok((image, image_memory))
}

unsafe fn transition_image_layout(
    device: &Device,
    data: &AppData,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<()> {
    let command_buffer = command_buffer::begin_single_time_commands(device, data)?;

    // one of the most common ways to perform layout transitions is using an image
    // memory barrier (that is a pipeline barrier, which is usually used to synchronize
    // access to resources); that can be used to transition image layouts and
    // transfer queue family ownership, when vk::SharingMode::Exclusive is used
    // there is an equivalent buffer memory barrier to do this for buffers

    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(0);

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED) // if the barrier is used to transfer queue family ownership, these fields should specify the queue family indices; if it is not used for this, they must be set to those values
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image) // specify the image, that is affected
        .subresource_range(subresource) // specify the specific part of the image, that is affected
        .src_access_mask(vk::AccessFlags::empty()) // TODO: which operations must happen before this barrier
        .dst_access_mask(vk::AccessFlags::empty()); // TODO: which operations must wait for the barrier

    // all types of pipline barriers as submitted using this function
    device.cmd_pipeline_barrier(
        command_buffer,
        vk::PipelineStageFlags::empty(), // TODO: in which pipeline stage do the operations occur, which must be performed before the barrier
        vk::PipelineStageFlags::empty(), // TODO: in which pipeline stage do the operations occur, which must wait for this barrier
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );

    command_buffer::end_single_time_commands(device, data, command_buffer)?;

    Ok(())
}
