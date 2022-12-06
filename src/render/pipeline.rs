#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

pub unsafe fn create_pipeline(device: &Device, data: &mut AppData) -> Result<()> {
    log::debug!("creating pipeline");

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

    // --- FIXED FUNCTION STAGE CONFIGURATION ---

    // this struct could store information about bindings and attribute descriptions
    // the vertex_binding_descriptions and vertex_attribute_descriptions would
    // take slices of structs describing this
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();

    // describe, which kind of geometry should be drawn from the vertex-data
    // and if 'primitive restart' should be enabled
    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // --- viewport configuration ---

    // define viewport (which region of the framebuffer will the output be rendered to)
    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(data.swapchain_extent.width as f32)
        .height(data.swapchain_extent.height as f32)
        .min_depth(0.0) // default
        .max_depth(1.0); // default

    // define scissor rectangle (pixels outside the scissor rectangle will be
    // discarded by the rasterizer)
    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(data.swapchain_extent);

    // create viewport state
    let viewports = &[viewport];
    let scissors = &[scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(viewports)
        .scissors(scissors);

    // --- rasterizer configuration ---

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false) // if this is enabled, fragments, that are beyond
        // near and far plane of the view frustum are clamped to the planes instead of
        // being discarded (useful for shadowmaps) -> requires enabling a GPU feature!
        .rasterizer_discard_enable(false) // if enabled, geometry never passes
        // through the rasterizer (hence, no output to framebuffer)
        .polygon_mode(vk::PolygonMode::FILL) // could also be LINE or POINT (but
        // requires GPU feature)-> fill area of polygon with fragments
        .line_width(1.0) // describe thickness of lines in terms of fragments (> 1 req. 'wide_lines' GPU feature)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE) // specify vertex order for faces to consider front-facing
        .depth_bias_enable(false); // could offset depth value based on slope of fragment, sometime used in shadowmapping

    // --- multisampling configuration ---

    // disable multisampling for now
    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::_1);

    // ignore depth and stencil buffers and tests for now

    // --- color blending configuration ---

    // this (basically) uses the following pseudocode:
    // if blend_enable {
    //     final_color.rgb = (src_color_blend_factor * new_color.rgb)
    //         <color_blend_op> (dst_color_blend_factor * old_color.rgb);
    //     final_color.a = (src_alpha_blend_factor * new_color.a)
    //         <alpha_blend_op> (dst_alpha_blend_factor * old_color.a);
    // } else {
    //     final_color = new_color;
    // }
    // final_color = final_color & color_write_mask;

    // use alpha blending:
    // final_color.rgb = new_alpha * new_color + (1 - new_alpha) * old_color;
    // final_color.a = new_alpha.a;
    let attachement = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::all())
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD);

    let attachements = &[attachement];

    // global configuration (allows to set blend constants to use in calculations)
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false) // could use this for bitwise combination (other
        // form of blending) -> will automatically disable first method of blending
        .logic_op(vk::LogicOp::COPY)
        .attachments(attachements)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    // could specify dynamic state here, which allows for configuration of specific parameters
    // on draw-time -> causes the configuration at compile time to be ignored!!

    // specify pipeline layout (could be used to pass uniforms or push-constants (i.e. arguments) to shader stages)
    // even though, we don't use this right now, we need to create an empty pipeline layout
    let layout_info = vk::PipelineLayoutCreateInfo::builder();

    data.pipeline_layout = device.create_pipeline_layout(&layout_info, None)?;

    let stages = &[vert_stage, frag_stage];
    let info = vk::GraphicsPipelineCreateInfo::builder()
        // programmable stages
        .stages(stages)
        // fixed function stage configurations
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&color_blend_state)
        // pipeline layout
        .layout(data.pipeline_layout)
        // render pass
        .render_pass(data.render_pass)
        .subpass(0); // "index of the subpass in the renderpass where this pipeline will be used"
                     // .base_pipeline_handle(vk::Pipeline::null()) // would be used to derive from another pipeline
                     // .base_pipeline_index(-1) // could be used to derive from another pipeline by idx

    data.pipeline = device
        .create_graphics_pipelines(
            vk::PipelineCache::null(), // could be used to reference a pipeline cache -> significantly speed up pipeline creation
            &[info],
            None,
        )?
        .0;

    info!("Created pipeline");

    device.destroy_shader_module(vert_shader_module, None);
    device.destroy_shader_module(frag_shader_module, None);

    Ok(())
}

unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
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
