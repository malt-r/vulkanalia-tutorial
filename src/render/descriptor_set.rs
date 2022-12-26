#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use nalgebra_glm as glm;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct UniformBufferObject {
    model: glm::Mat4,
    view: glm::Mat4,
    proj: glm::Mat4,
}

pub unsafe fn create_descriptor_set_layout(device: &Device, data: &mut AppData) -> Result<()> {
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
