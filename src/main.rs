// TODO: refactor (split up in multiple files)
#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

// winit related imports (window abstraction)
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

// vulkan imports for instance creation
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::window as vk_window;

// suface
use vulkanalia::vk::KhrSurfaceExtension;

use vulkanalia::vk::KhrXcbSurfaceExtension; // linux specific

// validation layer related imports
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;

// swapchain
use vulkanalia::vk::KhrSwapchainExtension;

use log::*;

use vulkanalia::vk::ExtDebugUtilsExtension;

use thiserror::Error;

// helper
use std::any::type_name;

const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

/// register a callback function, which will be called from the vulkan library,
/// if a validation layer message is sent
///
/// the "system" part of the declaration will select whatever calling convention
/// is the right one for interacting with the libraries of the current target
///
/// this function signature needs to match the following function:
/// https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/PFN_vkDebugUtilsMessengerCallbackEXT.html
extern "system" fn debug_callback(
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

fn print_type_of<T: ?Sized>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    //print_type_of(DEVICE_EXTENSIONS);

    // Create window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkanalia Tutorial")
        // using the logical size will be dpi-scaled
        .with_inner_size(LogicalSize::new(1024, 768))
        .build(&event_loop)?;

    let mut app = unsafe { App::create(&window)? };
    let mut destroying = false;
    event_loop.run(move |event, _, control_flow| {
        // poll for events, even if none is available
        *control_flow = ControlFlow::Poll;

        match event {
            // render a new frame, if all events other than the RequestRequested have
            // been cleared
            Event::MainEventsCleared if !destroying => unsafe { app.render(&window) }.unwrap(),
            // emitted, if the OS sends an event to the winit window (specifically
            // a request to close the window)
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                destroying = true;
                *control_flow = ControlFlow::Exit;
                log::debug!("Exit...");
                unsafe {
                    app.destroy();
                }
            }
            _ => {}
        }
    });
}

/// creates a new vulkan instance using entry.create_instance
/// the window parameter is used to enumerate all required extensions
///
/// The 'Instance' returned by this function is not a raw vulkan instance
/// (this would be vk::Instance), it is an abstraction created by vulkanalia,
/// which combines the raw vulkan instance and the loaded commands for that instance
unsafe fn create_instance(window: &Window, entry: &Entry, data: &mut AppData) -> Result<Instance> {
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

    if VALIDATION_ENABLED {
        debug!("Setting up validation layers");
    }

    if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
        return Err(anyhow!("Validation layer requested but not supported."));
    }

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
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
    if VALIDATION_ENABLED {
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
        .user_callback(Some(debug_callback));

    if VALIDATION_ENABLED {
        trace!("Pushing debug_info to InstanceCreateInfo::pnext");
        // this does not seem to need a mutable instance of info..
        // this is pretty odd, because push_next will modify the internals of
        // info, maybe this is related to the above TODO
        info.push_next(&mut debug_info);
    }

    let instance = entry.create_instance(&info, None)?;

    if VALIDATION_ENABLED {
        // register the debug messenger and store the result in AppData
        data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
    }

    Ok(instance)
}

#[derive(Clone, Debug)]
struct App {
    entry: Entry,
    instance: Instance,
    data: AppData,
    device: Device,
}

// The error macro of the thiserror-crate enables definition of custom
// error types in terms of structs or enums without all the boilerplate code
// which is required for implementing std::error::Error
//
// A custom error message can be defined with a format string, which uses members
// of data structure
#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);

unsafe fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()> {
    for device in instance.enumerate_physical_devices()? {
        let properties = instance.get_physical_device_properties(device);
        trace!("Checking physical device {}", properties.device_name);
        if let Err(error) = check_physical_device(instance, data, device) {
            warn!("Skipping physical device ('{}'): {}", properties.device_name, error)
        } else {
            info!("Selecting physical device ('{}')", properties.device_name);
            data.physical_device = device;
            return Ok(());
        }
    }
    Err(anyhow!("Failed to select a physical device"))
}

#[derive(Debug, Copy, Clone)]
pub struct QueueFamilyIndices {
    graphics: u32,
    presentation: u32,
}

impl QueueFamilyIndices {
    /// gets queue familiy indices for specified vulkan instance and physical device;
    /// can't be constant, because these indices may vary from device to device
    unsafe fn get(instance: &Instance, data: &AppData, physical_device: vk::PhysicalDevice)
        -> Result<Self> {
            let properties = instance.get_physical_device_queue_family_properties(physical_device);

            // look for the first queue family, which supports the GRAPHICS property
            let graphics_property =
                properties
                .iter()
                .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .map(|i| i as u32);

            let mut present = None;

            // find the queue family, which supports presentation
            for (index, properties) in properties.iter().enumerate() {
                if instance.get_physical_device_surface_support_khr(
                    physical_device,
                    index as u32,
                    data.surface
                    )? {
                    present = Some(index as u32);
                    break;
                }
            }

            if let (Some(graphics), Some(present)) = (graphics_property, present) {
                Ok(Self {graphics, presentation: present})
            } else {
                Err(anyhow!(SuitabilityError("Missing required queue families.")))
            }
        }
}

// we need to check, whether a given physical device
// is suitable to use for our needs
unsafe fn check_physical_device(
    instance: &Instance,
    data: &AppData,
    physical_device: vk::PhysicalDevice
) -> Result<()> {
    QueueFamilyIndices::get(instance, data, physical_device)?;
    check_physical_device_extensions(instance, physical_device)?;
    Ok(())
}

unsafe fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    ) -> Result<()> {
    let extensions = instance
        .enumerate_device_extension_properties(physical_device, None)?
        .iter()
        .map(|e| e.extension_name)
        .collect::<HashSet<_>>();

    // TODO: print, which one is missing
    if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
        Ok(())
    } else {
        Err(anyhow!(SuitabilityError("Missing required device extensions.")))
    }
}

unsafe fn create_logical_device(instance: &Instance, data: &mut AppData) -> Result<Device> {
    // specify queues to be created
    let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

    let mut unique_indices = HashSet::new();
    unique_indices.insert(indices.graphics);
    unique_indices.insert(indices.presentation);

    // the queue priorities specify the prio of a queue for scheduling of
    // command execution
    //
    // From the tutorial:
    // "The current drivers will only allow you to create a small number of
    // queues for each queue family and you don't really need more than one."
    // That's because you can create all of the command buffers on multiple threads
    // and then submit them all at once from the main thread with a single low-overhead call"
    let queue_priorities = &[1.0];
    let queue_infos =
        unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*i)
                    .queue_priorities(queue_priorities)
        }).collect::<Vec<_>>();

    // enable device specific layers
    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // specify used device features (queried for in check_physical_device)
    // TODO: nothing special required for now, specify later
    let features = vk::PhysicalDeviceFeatures::builder();

    // convert device_extension Strings to null terminated strings
    let extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    // create the logical device
    // TODO: what is this?
    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);
    let device = instance.create_device(data.physical_device, &info, None)?;

    // get handle to the graphics queue
    data.graphics_queue = device.get_device_queue(indices.graphics, 0);
    data.present_queue = device.get_device_queue(indices.presentation, 0);

    trace!("graphics queue family index: {}", indices.graphics);
    trace!("presentation queue family index: {}", indices.presentation);

    Ok(device)
}

// TODO: expose own safe wrapper around vulkan calls, which asserts the calling
// of the correct invariants of the vulkan API functions
impl App {
    /// creates the app
    unsafe fn create(window: &Window) -> Result<Self> {
        // create library loader for the vulkan library (LIBRARY is a constant
        // path pointing to the Vulkan library)
        // this loads initial Vulkan commands from the library
        let loader = LibloadingLoader::new(LIBRARY)?;
        // load the entry point of the Vulkan library using the loader
        let entry = Entry::new(loader).map_err(|e| anyhow!("{}", e))?;
        // use the window and entry to create a vulkan instance
        let mut data = AppData::default();
        let instance = create_instance(window, &entry, &mut data)?;

        // setup window surface
        data.surface = vk_window::create_surface(&instance, window)?;

        pick_physical_device(&instance, &mut data)?;
        let device = create_logical_device(&instance, &mut data)?;
        Ok(Self {
            entry,
            instance,
            data,
            device,
        })
    }

    /// renders one frame
    unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// destroy the app
    unsafe fn destroy(&mut self) {
        // None is for allocation callbacks
        self.device.destroy_device(None);

        // if validation is enabled, the debug messenger needs to be destroyed,
        // before the instance is destroyed
        if VALIDATION_ENABLED {
            self.instance
                .destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        self.instance.destroy_surface_khr(self.data.surface, None);
        // be explicit about it
        self.instance.destroy_instance(None);
    }
}

#[derive(Clone, Debug, Default)]
struct AppData {
    surface: vk::SurfaceKHR,
    messenger: vk::DebugUtilsMessengerEXT,
    // this will be implicitly destroyed, if the instance is destroyed,
    // so no further handling of this in App::destroy() required
    physical_device: vk::PhysicalDevice,

    // queues, which will be created along with logic device creation
    // queues are implicitly cleaned up, when the device is destroyed
    graphics_queue: vk::Queue,

    // the presentation queue also needs to be created with the logic
    // device
    present_queue: vk::Queue,
}
