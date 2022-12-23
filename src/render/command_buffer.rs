use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::{app::AppData, render::pipeline};
use log::info;

pub unsafe fn create_command_buffers(device: &Device, data: &mut AppData) -> Result<()> {
    debug_assert!(data.framebuffers.len() > 0);

    log::debug!("Creating {} command buffers", data.framebuffers.len());

    // as drawing requires binding of the correct framebuffer, we create a command
    // buffer for each one
    let allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(data.command_pool)
        // PRIMARY:     can be submitted to queue directly, but can't be called from other
        //              command buffers
        // SECONDARY:   can't be submitted directly to queue, but can be called from
        //              primary command buffers
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(data.framebuffers.len() as u32);

    data.command_buffers = device.allocate_command_buffers(&allocate_info)?;

    // record command buffers
    for (i, command_buffer) in data.command_buffers.iter().enumerate() {
        // begin command buffer
        let inheritance = vk::CommandBufferInheritanceInfo::builder();

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::empty()) // could further specify usage of command buffer (ONE_TIME_SUBMIT etc.)
            .inheritance_info(&inheritance);

        device.begin_command_buffer(*command_buffer, &begin_info)?;

        // start command buffer

        // define render area (where data should be loaded and stored during render operations)
        // pixels outside of this area will be undefined -> should match extent
        // of framebuffer images for best performance
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D::default())
            .extent(data.swapchain_extent);

        let color_clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        // begin render pass
        let clear_values = &[color_clear_value];
        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(data.render_pass)
            .framebuffer(data.framebuffers[i])
            .render_area(render_area)
            .clear_values(clear_values);

        device.cmd_begin_render_pass(
            *command_buffer,
            &info,
            // inline: render pass commands will be provided by primary command buffer
            // secondary: render pass commands will be provided in secondary command buffer(s)
            vk::SubpassContents::INLINE,
        );

        // bind pipeline -> tells vulkan, which attachments to use
        device.cmd_bind_pipeline(
            *command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            data.pipeline,
        );

        // bind vertex buffer
        device.cmd_bind_vertex_buffers(*command_buffer, 0, &[data.vertex_buffer], &[0]);

        // bind index buffer (you can only have one index buffer)
        device.cmd_bind_index_buffer(*command_buffer, data.index_buffer, 0, vk::IndexType::UINT16);

        // draw (indexed)
        device.cmd_draw_indexed(
            *command_buffer,
            // TODO: this needs to match the total vertex count.. as the whole
            // vertex buffer thing will be encapsulated later in a struct anyway
            // don't care right now
            pipeline::INDICES.len() as u32, // index count
            1,                              // instance count
            0,                              // first index
            0,                              // vertex offset -> start at zeroth one
            0,                              // first instance
        );

        // finishing up
        device.cmd_end_render_pass(*command_buffer);
        device.end_command_buffer(*command_buffer)?;
        log::debug!("Created command buffer");
    }

    Ok(())
}
