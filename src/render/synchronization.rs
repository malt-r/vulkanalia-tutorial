use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;
use crate::app::MAX_FRAMES_IN_FLIGHT;

pub unsafe fn create_sync_objects(device: &Device, data: &mut AppData) -> Result<()> {
    // currently, creating semaphores does not require any specific flags
    let sem_info = vk::SemaphoreCreateInfo::builder();
    let fence_info = vk::FenceCreateInfo::builder()
        .flags(vk::FenceCreateFlags::SIGNALED); // create in signaled stage, otherwise,
                                                // we initially wait forever, because the fence
                                                // never was used...

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        data.image_ready_semaphores.push(device.create_semaphore(&sem_info, None)?);
        data.render_finished_semaphores.push(device.create_semaphore(&sem_info, None)?);

        data.in_flight_fences.push(device.create_fence(&fence_info, None)?);
    }

    data.images_in_flight = data.swapchain_images
        .iter()
        .map(|_| vk::Fence::null())
        .collect();

    Ok(())
}
