use anyhow::{anyhow, Result};

use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

// attachments specified in render pass creation are bound by wrapping them into
// vk::Framebuffer objects -> references an vk::ImageView, that represents the attachment
//
// The image we use for attachment depends on which image is returned by the swapchain
// if we retrieve one for presentation -> create framebuffers for each imageView!
pub unsafe fn create_framebuffers(device: &Device, data: &mut AppData) -> Result<()> {
    log::debug!("creating framebuffers");

    debug_assert!(data.swapchain_image_views.len() > 0);

    data.framebuffers = data
        .swapchain_image_views
        .iter()
        .map(|i| {
            // use imageview as attachment
            let attachments = &[*i];
            let create_info = vk::FramebufferCreateInfo::builder()
                // render pass with which this framebuffer needs to be compatible with
                // -> roughly means same number and type of attachments
                .render_pass(data.render_pass)
                .attachments(attachments)
                // define dimensions of the framebuffer
                .width(data.swapchain_extent.width)
                .height(data.swapchain_extent.height)
                .layers(1);
            device.create_framebuffer(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    debug_assert!(data.framebuffers.len() > 0);

    Ok(())
}
