// loggin
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};
use log::*;

use std::ffi::CStr;
use std::os::raw::c_void;
use vulkanalia::vk;

pub const ENABLED: bool = cfg!(debug_assertions);

pub const LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

/// register a callback function, which will be called from the vulkan library,
/// if a validation layer message is sent
///
/// the "system" part of the declaration will select whatever calling convention
/// is the right one for interacting with the libraries of the current target
///
/// this function signature needs to match the following function:
/// https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/PFN_vkDebugUtilsMessengerCallbackEXT.html
pub extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    // can be "general", "validation" or "performance"
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    // will be specified during setup, allows the user to pass in their own data
    _: *mut c_void,
) -> vk::Bool32 {
    // TODO: is there a way to add a nullptr check here?
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    // convert severity to according logging mode
    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        debug!("({:?}) {}", type_, message);
    } else {
        trace!("({:?}) {}", type_, message);
    }

    // the return value of this function is interpreted as an indication, if
    // the operation, which led to this debug_callback should be aborted
    vk::FALSE
}
