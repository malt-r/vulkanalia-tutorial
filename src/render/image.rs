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

// TODO: all helper methods that submit command buffers do that synchronously,
// by waiting for the queue to become idle; practical applications should combine
// these operations in a single command buffer and execute them asynchronously for
// higher throughput
// -> setup_command_buffer, that the helper functions record commands into and add a
//    flush_setup_commands, to execute the commands that have been recorded so far
//    It's best to do this after the texture mapping works to check if the texture
//    resources are still set up correctly
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

    // transition the texture image to vk::ImageLayout::TRANSFER_DST_OPTIMAL
    transition_image_layout(
        device,
        data,
        data.texture_image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageLayout::UNDEFINED, // image was defined with this layout, so we should pass it as the old layout
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    )?;

    // execute the buffer to image copy operation
    copy_buffer_to_image(
        device,
        data,
        staging_buffer,
        data.texture_image,
        width,
        height,
    )?;

    // to be able to start sampling from the image, we need to transition it to prepare for shader access
    transition_image_layout(
        device,
        data,
        data.texture_image,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    )?;

    // cleanup
    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

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

//  TODO: besser verstehen
// need to handle two transitions:
// - undefined -> transfer destination (transfer writes, that don't need to wait on anything)
// - transfer destination -> shader reading (shader reads should wait on transfer writes, specifically the shader reads in the fragment shader)
unsafe fn transition_image_layout(
    device: &Device,
    data: &AppData,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<()> {
    // Note: check this table for reference: https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#synchronization-access-types-supported
    // TODO: what is the differnece between access masks and stage masks
    let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE, // transfer writes must occur in the pipeline transfer stage
                vk::PipelineStageFlags::TOP_OF_PIPE, // the writes don't have to wait on anything, so specify earliest possible stage TOP_OF_PIPE
                vk::PipelineStageFlags::TRANSFER,
            ),
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            _ => return Err(anyhow!("Unsupported image layout transition!")),
        };

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
        .layer_count(1);

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED) // if the barrier is used to transfer queue family ownership, these fields should specify the queue family indices; if it is not used for this, they must be set to those values
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image) // specify the image, that is affected
        .subresource_range(subresource) // specify the specific part of the image, that is affected
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);

    // all types of pipline barriers as submitted using this function
    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage_mask,
        dst_stage_mask,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );

    command_buffer::end_single_time_commands(device, data, command_buffer)?;

    Ok(())
}

pub unsafe fn copy_buffer_to_image(
    device: &Device,
    data: &AppData,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
) -> Result<()> {
    let command_buffer = command_buffer::begin_single_time_commands(device, data)?;

    // specify, which parts of the buffer are going to be copied to which parts of the image
    let subresource = vk::ImageSubresourceLayers::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1);

    let region = vk::BufferImageCopy::builder()
        .buffer_offset(0) // byte offset in the buffer, at which pixel values start
        .buffer_row_length(0) // row_length and image_height specify, how pixels are laid out in memory (could have some padding bytes; 0 signals, that pixels are tightly packed)
        .buffer_image_height(0)
        .image_subresource(subresource)
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        });

    device.cmd_copy_buffer_to_image(
        command_buffer,
        buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL, // indicates, which layout the image is currently using
        &[region], // it's possible to specify an array of vk::BufferImageCopy to perform many different copies from this buffer to the image in one operation
    );

    command_buffer::end_single_time_commands(device, data, command_buffer)?;
    Ok(())
}

pub(crate) unsafe fn create_image_view(
    device: &Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView> {
    let components = vk::ComponentMapping::builder()
        .r(vk::ComponentSwizzle::IDENTITY)
        .g(vk::ComponentSwizzle::IDENTITY)
        .b(vk::ComponentSwizzle::IDENTITY)
        .a(vk::ComponentSwizzle::IDENTITY);

    create_image_view_with_components(device, image, format, components.build())
}

pub(crate) unsafe fn create_image_view_with_components(
    device: &Device,
    image: vk::Image,
    format: vk::Format,
    components: vk::ComponentMapping,
) -> Result<vk::ImageView> {
    // define subresource range -> describe purpose and which parts of
    // image should be accessed
    // we use the images as color targets and don't use mipmaps or multiple layers
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    // create image view create info..
    let info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::_2D) // specifies, how the image data should be interpreted, allows to treat images as 1D, 2D, 3D and cube maps
        .format(format)
        .subresource_range(subresource_range)
        .components(components);

    Ok(device.create_image_view(&info, None)?)
}

pub(crate) unsafe fn create_texture_image_view(device: &Device, data: &mut AppData) -> Result<()> {
    data.texture_image_view =
        create_image_view(device, data.texture_image, vk::Format::R8G8B8A8_SRGB)?;
    Ok(())
}

// Note: a sampler does not reference an image directly, but it can be applied to
// any image, we want
pub unsafe fn create_texture_sampler(device: &Device, data: &mut AppData) -> Result<()> {
    let info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR) // how to interpolate texels that are magnified (oversampling)
        .min_filter(vk::Filter::LINEAR) // how to interpolate texels that are minified (undersampling)
        .address_mode_u(vk::SamplerAddressMode::REPEAT) // address mode for x (in texel space this is u)
        .address_mode_v(vk::SamplerAddressMode::REPEAT) // address mode for y (in texel space this is v)
        .address_mode_w(vk::SamplerAddressMode::REPEAT) // address mode for z (in texel space this is w)
        .anisotropy_enable(true) // TODO: yeah, but what IS anisotropy exactly?
        .max_anisotropy(16.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false) // if set to true, we can adress texels in [0, width) range, if normalized, then [0, 1)
        .compare_enable(false) // if comparison function is enabled, texels will first be compared to a value and the result of that comp is used in filtering operations
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR) // TODO: look at that later
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0);
    Ok(())
}
