use anyhow::{anyhow, Result};

use log::info;
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

pub unsafe fn create_render_pass(
    insntance: &Instance,
    device: &Device,
    data: &mut AppData,
) -> Result<()> {
    // specify render pass: which framebuffer attachements are used while rendering
    // - how many color buffers?
    // - how many depth buffers?
    // - how many samples for each of them?
    // - how to handle buffer contents?

    let color_attachment = vk::AttachmentDescription::builder()
        .format(data.swapchain_format)
        .samples(vk::SampleCountFlags::_1) // no multisampling yet
        // load op and store op apply to color and depth
        .load_op(vk::AttachmentLoadOp::CLEAR) // what to do before rendering
        .store_op(vk::AttachmentStoreOp::STORE) // what to do after rendering
        // stencil specific ops
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE) // ignore for now
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE) // ignore for now
        // images need to be transitioned to a layout, which is suitable for
        // the operation that they're going to be involved in next
        .initial_layout(vk::ImageLayout::UNDEFINED) // don't care, what layout the image is before this
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR); // present image to swapchain next

    // --- define subpasses ---

    // every subpass references one or more attachments
    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0) // which attachment in the attachments-array is referenced by this reference?
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL); // which layout should the attachment during the subpass

    // the index of the attachment in this array is directly referenced in the
    // fragment shader by the `layout(location = 0) out vec4 outColor`
    let attachment_references = &[color_attachment_ref];

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(attachment_references); // there are other attachment-types!

    // --- define render pass ---
    let attachements = &[color_attachment];
    let subpasses = &[subpass];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(attachements)
        .subpasses(subpasses);

    data.render_pass = device.create_render_pass(&info, None)?;

    Ok(())
}
