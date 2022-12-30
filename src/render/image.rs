#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use std::fs::File;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use crate::{app::AppData, render::buffer};

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

    // create vk::image
    let info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::_2D) // what kind of coordinate system to adress texels?
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        }) // dimensions of image
        .mip_levels(1) // no mipmapping for now
        .array_layers(1) // image is not an array
        .format(vk::Format::R8G8B8A8_SRGB) // same as texels in the staging buffer
        .tiling(vk::ImageTiling::OPTIMAL) // LINEAR: texels laid out in row-major order (like pixels array); OPTIMAL: texels laid out in implementation defined order for optimal access
        .initial_layout(vk::ImageLayout::UNDEFINED) // UNDEFINED: not usable by GPU and texels will discarded on first transition (does not matter for use, because we will transition the image to be a destination for copy from buffer object)
        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
        .samples(vk::SampleCountFlags::_1) // only relevant for images, which are used as attachements (so stick to one sample); some further options are used for sparse images (where only certain ranges are backed by memory - e.g. to store voxel terrain data)
        .sharing_mode(vk::SharingMode::EXCLUSIVE); // will only be used by one queue family

    data.texture_image = device.create_image(&info, None)?;

    // TODO: continue

    Ok(())
}
