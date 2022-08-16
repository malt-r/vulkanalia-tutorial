use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;

use crate::app::AppData;

pub unsafe fn create_sync_objects(device: &Device, data: &mut AppData) -> Result<()> {
    // currently, creating semaphores does not require any specific flags
    let sem_info = vk::SemaphoreCreateInfo::builder();

    data.image_ready_sem = device.create_semaphore(&sem_info, None)?;
    data.render_finished_sem = device.create_semaphore(&sem_info, None)?;

    Ok(())
}
