#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

pub unsafe fn create_pipeline(device: &Device, data: &mut AppData) -> Result<()> {
    let vert = include_bytes!("../../shaders/vert.spv");
    let frag = include_bytes!("../../shaders/frag.spv");

    let vert_shader_module = create_shader_module(device, vert)?;
    let frag_shader_module = create_shader_module(device, frag)?;

    // assign shaders to specific pipeline stage with vk::PipelineShaderStageCreateInfo
    // NOTE: this features a member specialization_info, which allows for passing
    // values for shader constants (more efficient than passing in runtime)
    let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX) // in which pipeline stage should we use it
        .module(vert_shader_module)
        .name(b"main\0"); // specify name of entrypoint -> it's possible to combine
        // multiple shaders in one bytecode file and reference different shaders in
        // pipeline creation

    let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module)
        .name(b"main\0");


    device.destroy_shader_module(vert_shader_module, None);
    device.destroy_shader_module(frag_shader_module, None);

    Ok(())
}

unsafe fn create_shader_module(
    device: &Device,
    bytecode: &[u8],
) ->Result<vk::ShaderModule> {
    // this will pass the bytecode to ShaderModuleCreateInfo, which expects an &[u32]
    // slice -> use slice::align_to to convert the &[u8], but have to make sure
    // that the slice matches the alignment requirements. We can't be sure of that,
    // so we create a Vec from it first

    let bytecode = Vec::<u8>::from(bytecode);
    let (prefix, code, suffix) = bytecode.align_to::<u32>();
    if !prefix.is_empty() || !suffix.is_empty() {
        return Err(anyhow!("Shader bytecode is not properly aligned."));
    }

    let info = vk::ShaderModuleCreateInfo::builder()
        .code_size(bytecode.len())
        .code(code);

    Ok(device.create_shader_module(&info, None)?)
}
