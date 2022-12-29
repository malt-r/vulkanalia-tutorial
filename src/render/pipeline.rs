#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

use log::info;
use vulkanalia::prelude::v1_0::*;

use super::{buffer, descriptor_set};
use crate::app::AppData;

// vertex data related
// needed, because glm hat not yet const functions
use lazy_static::lazy_static;

// define the data
use nalgebra_glm as glm;

// used to calculate the size of vertex data
use std::mem::size_of;

// memcpy
use std::ptr::copy_nonoverlapping as memcpy;

// repr annotation states to use other memory layout strategies (C in this case)
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pos: glm::Vec2,
    color: glm::Vec3,
}

impl Vertex {
    fn new(pos: glm::Vec2, color: glm::Vec3) -> Self {
        Self { pos, color }
    }

    // needed to tell vulkan, how to pass vertex data to the shader
    // Vertex binding describes, at which rate to laod data from memory
    // specifies number of bytes between data entries whether to move to next entry
    // after each vertex or instance
    fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            // specifies the index of the binding in array of bindings
            .binding(0)
            // specifies the number of bytes from one entry to the next
            .stride(size_of::<Vertex>() as u32)
            // either vertex or instance (for instanced rendering)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    // used to specify how to handle vertex input
    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        let pos = vk::VertexInputAttributeDescription::builder()
            // from which binding does the per-vertex data come?
            .binding(0)
            // references the location directive of the attribute in shader code
            .location(0)
            // describes the type of data of the attribute (confusingly the same enum as color formats)
            // in this case: vec2 has two 32 bit floats
            // should also bitwidth of the datatypes
            .format(vk::Format::R32G32_SFLOAT)
            // number of bytes since the start of the per vertex data
            .offset(0)
            .build();

        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<glm::Vec2>() as u32)
            .build();

        [pos, color]
    }
}

// glm provied rust-types that exactly match shader vector types
// combining position and color in one array (different attributes in one array)
// is also known as INTERLEAVING ATTRIBUTES
/*lazy_static! {
    static ref VERTICES: Vec<Vertex> = vec![
        Vertex::new(glm::vec2(0.0, -0.5), glm::vec3(1.0, 1.0, 1.0)),
        Vertex::new(glm::vec2(0.5, 0.5), glm::vec3(0.0, 1.0, 0.0)),
        Vertex::new(glm::vec2(-0.5, 0.5), glm::vec3(0.0, 0.0, 1.0)),
    ];
}*/

lazy_static! {
    static ref VERTICES: Vec<Vertex> = vec![
        Vertex::new(glm::vec2(-0.5, -0.5), glm::vec3(1.0, 0.0, 0.0)),
        Vertex::new(glm::vec2(0.5, -0.5), glm::vec3(0.0, 1.0, 0.0)),
        Vertex::new(glm::vec2(0.5, 0.5), glm::vec3(0.0, 0.0, 1.0)),
        Vertex::new(glm::vec2(-0.5, 0.5), glm::vec3(1.0, 1.0, 1.0)),
    ];
}

// indices for drawing two triangles with vertex data
// this also needs to be uploaded into an vk::Buffer
pub const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

// buffers are regions of memory used for storage of arbitraty data and can
// be read by the graphics card
// buffer allocation needs to be performed explicitly (they do not allocate memory
// for themselves)
pub unsafe fn create_vertex_buffer(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let size = (size_of::<Vertex>() * VERTICES.len()) as u64;

    // use staging buffer to store the vertex data and transfer it later to
    // the actual vertex buffer
    let (staging_buffer, staging_buffer_memory) = buffer::create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC, // buffer can be used as source in a memory transfer operation
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // fill the staging buffer -> map cpu memory to the staging_buffer_memory
    let memory = device.map_memory(
        staging_buffer_memory,
        0, // no offset
        size,
        vk::MemoryMapFlags::empty(),
    )?;

    memcpy(VERTICES.as_ptr(), memory.cast(), VERTICES.len());
    device.unmap_memory(staging_buffer_memory);

    // create a vertex buffer and memory in device_local memory
    let (vertex_buffer, vertex_buffer_memory) = buffer::create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    // copy from staging buffer to vertex buffer
    buffer::copy_buffer(device, data, staging_buffer, vertex_buffer, size)?;

    // release temporary resources
    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

    data.vertex_buffer = vertex_buffer;
    data.vertex_buffer_memory = vertex_buffer_memory;

    // the driver may not copy the data immediately into the buffer memory
    // two ways to deal with this:
    // - use a memory heap that is host coherent (our approach)
    // - call flush_mapped_memory_ranges after writing to memory and call
    //   invalidate_mapped_memory_ranges before reading from mapped memory
    // flushing memory ranges or using coherent memory ranges means, that the
    // driver is aware of our changes, but it is not visible to the gpu yet;
    // spec tells us, that the changes will be completed in the next call to
    // queue_submit

    Ok(())
}

pub unsafe fn create_index_buffer(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    let size = (size_of::<u16>() * INDICES.len()) as u64;

    log::debug!("Index buffer size: {}", size);

    // use staging buffer to store the vertex data and transfer it later to
    // the actual vertex buffer
    let (staging_buffer, staging_buffer_memory) = buffer::create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC, // buffer can be used as source in a memory transfer operation
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
    )?;

    // fill the staging buffer -> map cpu memory to the staging_buffer_memory
    let memory = device.map_memory(
        staging_buffer_memory,
        0, // no offset
        size,
        vk::MemoryMapFlags::empty(),
    )?;

    memcpy(INDICES.as_ptr(), memory.cast(), INDICES.len());
    device.unmap_memory(staging_buffer_memory);

    // create an index buffer and memory in device_local memory
    let (index_buffer, index_buffer_memory) = buffer::create_buffer(
        instance,
        device,
        data,
        size,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    data.index_buffer = index_buffer;
    data.index_buffer_memory = index_buffer_memory;

    // copy from staging buffer to vertex buffer
    buffer::copy_buffer(device, data, staging_buffer, index_buffer, size)?;

    // release temporary resources
    device.destroy_buffer(staging_buffer, None);
    device.free_memory(staging_buffer_memory, None);

    Ok(())
}

pub unsafe fn create_uniform_buffers(
    instance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    data.uniform_buffers.clear();
    data.uniform_buffers_memory.clear();

    // we will update the uniform buffers contents each frame; as we have multiple
    // frames in flight, we could either create one uniform buffer for each frame
    // or for each swapchain image; the uniform buffer is refered to by the command
    // buffer (which is per swapchain image), so we create one uniform buffer for
    // each swapchain image as well
    for _ in 0..data.swapchain_images.len() {
        let (uniform_buffer, uniform_buffer_memory) = buffer::create_buffer(
            instance,
            device,
            data,
            size_of::<descriptor_set::UniformBufferObject>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
        data.uniform_buffers.push(uniform_buffer);
        data.uniform_buffers_memory.push(uniform_buffer_memory);

        // we will create a specific function, which updates the buffer with
        // new data every frame, so there is no need for map_memory here
    }

    Ok(())
}

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

    // prepare pipeline to accept vertex data
    let binding_descriptions = &[Vertex::binding_description()];
    let attribute_descriptions = Vertex::attribute_descriptions();

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

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
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE) // specify vertex order for faces to consider front-facing;
        // because of the y-sign flip, we did in the update_uniform_buffers (to update the
        // model view proj matrix, vertices are drawn in counter-clockwise direction,
        // which needs to be reflected here)
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

    // reference descriptor set layouts used in the pipeline
    let set_layouts = &[data.descriptor_set_layout];
    // specify pipeline layout (could be used to pass uniforms or push-constants (i.e. arguments) to shader stages)
    // even though, we don't use this right now, we need to create an empty pipeline layout
    let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(set_layouts);

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
