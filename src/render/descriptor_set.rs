use std::mem::size_of;

#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use nalgebra_glm as glm;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

// vulkan has very specific alignment requirements for structs passed as
// UBOs to a shader (see: https://www.khronos.org/registry/vulkan/specs/1.1-extensions/html/chap14.html#interfaces-resources-layout)
// and include padding as required
// and obsiously, the structure layout has to match the specified layout in
// the shader
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) struct UniformBufferObject {
    pub(crate) model: glm::Mat4,
    pub(crate) view: glm::Mat4,
    pub(crate) proj: glm::Mat4,
}

pub unsafe fn create_descriptor_set_layout(device: &Device, data: &mut AppData) -> Result<()> {
    log::debug!("Creating descriptor set layout");
    // leave immutable_sampler field at default(used for image sampling)
    let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0) // reference to vertex shader
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1) // it is possible for a shader variable to represent an array of uniform buffer objects (number specified by this count) -> could be used to specify transform for each bone in skeletal animation
        .stage_flags(vk::ShaderStageFlags::VERTEX); // in which stage of the graphics pipeline

    let bindings = &[ubo_binding];
    let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

    data.descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

    Ok(())
}

pub unsafe fn create_descriptor_sets(device: &Device, data: &mut AppData) -> Result<()> {
    log::debug!("Creating descriptor sets");
    // allocate descriptor sets (with defined layout) from descriptor pool
    let layouts = vec![data.descriptor_set_layout; data.swapchain_images.len()];

    // create a descriptor set with the same layout for each swapchain image
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(data.descriptor_pool)
        .set_layouts(&layouts);

    // descriptor sets will be freed, once the descriptor_pool is destroyed
    data.descriptor_sets = device.allocate_descriptor_sets(&info)?;

    // the descriptors inside the newly created sets need to be configured
    for i in 0..data.swapchain_images.len() {
        log::debug!("Updating descriptor set with index {}", i);
        // create descriptor buffer info
        let info = vk::DescriptorBufferInfo::builder()
            .buffer(data.uniform_buffers[i]) // which buffer to bind
            .offset(0) // offset in the buffer
            .range(size_of::<UniformBufferObject>() as u64); // how long is the range in the buffer

        // descriptor sets are updated with a writeDescriptorSet-struct
        let buffer_info = &[info];
        let ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(data.descriptor_sets[i])
            .dst_binding(0) // we gave our uniform buffer binding index 0, so reference it here
            .dst_array_element(0) // descriptors can be array, so specify the first index in the array of the element, we want to update
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(buffer_info); // for descriptors, which refer to image data, this would be 'image_info'

        // actually update the descriptor set
        device.update_descriptor_sets(&[ubo_write], &[] as &[vk::CopyDescriptorSet]);
    }

    Ok(())
}
