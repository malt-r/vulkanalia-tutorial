// loggin
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};
use log::*;

// std
use std::collections::HashSet;

use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ExtDebugUtilsExtension;
use vulkanalia::window as vk_window;

use winit::window::Window;

use super::validation;
use crate::app::AppData;

/// creates a new vulkan instance using entry.create_instance
/// the window parameter is used to enumerate all required extensions
///
/// The 'Instance' returned by this function is not a raw vulkan instance
/// (this would be vk::Instance), it is an abstraction created by vulkanalia,
/// which combines the raw vulkan instance and the loaded commands for that instance
pub unsafe fn create_instance(
    window: &Window,
    entry: &Entry,
    data: &mut AppData,
) -> Result<Instance> {
    // no strictly necessary
    let application_info = vk::ApplicationInfo::builder()
        .application_name(b"Vulkan Tutorial\0")
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(b"No Engine\0")
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 0, 0));

    // check for availability of validation layers
    //
    // seems to be a rust idiom: in order to extract a property of objects in a
    // collection, get the iterator, use the map-function, to 'remap' the object
    // to the property and collect all remapped elements in another collection
    let available_layers = entry
        .enumerate_instance_layer_properties()?
        .iter()
        .map(|l| l.layer_name)
        .collect::<HashSet<_>>();

    if validation::ENABLED {
        debug!("Setting up validation layers");
    }

    if validation::ENABLED && !available_layers.contains(&validation::LAYER) {
        return Err(anyhow!("Validation layer requested but not supported."));
    }

    let layers = if validation::ENABLED {
        vec![validation::LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    // lots of information is passed to vulkan (and vulkanalia) by passing structs
    // so for creating an instance, we need to fill in one more struct
    //
    // enumerate all globally required extensions for vk_window and convert them to
    // null terminated c_strings (*const i8)
    //
    // globally means global for the whole program
    let mut extensions = vk_window::get_required_instance_extensions(window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    // this extension is needed to set up a custom debug messenger with custom
    // message callback for validation layers
    if validation::ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    // create a vulkan instance (the connection between our program and the
    // Vulkan library)
    let mut info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions);

    let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
        // TODO: this does not work. Normal validation works,
        // tested by removing destroy call to debug messenger before
        // destroying instance
        .user_callback(Some(validation::debug_callback));

    if validation::ENABLED {
        trace!("Pushing debug_info to InstanceCreateInfo::pnext");
        // this does not seem to need a mutable instance of info..
        // this is pretty odd, because push_next will modify the internals of
        // info, maybe this is related to the above TODO
        info.push_next(&mut debug_info);
    }

    let instance = entry.create_instance(&info, None)?;

    if validation::ENABLED {
        // register the debug messenger and store the result in AppData
        data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
    }

    Ok(instance)
}
