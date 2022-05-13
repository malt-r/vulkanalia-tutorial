# Various bits and notes about vulkan #

## winit ##

The winit::event_loop::EventLoop offers a way to retrieve events from the system.
Calling ::new() will initialize the "context" and everything, that is required for
receiving events.

## memory management ##

All vulkan API calls, which allocate memory (by creating some kind of object),
accept an `allocator` parameter, by which a custom allocator can be passed.
This should probably be used to monitor ressource usage later on.

## error handling / validation layers ##

Vulkan is designed with minimal driver overhead in mind, so even basic error
handling is omitted. Because of that, it's easy to make many small mistakes,
which just lead to crashes. Optionally error checks can be hooked into many
vulkan operations by using validation layers. (Used for checking values of parameters,
tracking creation and destruction of objects and find ressource leaks, etc.)

Validation layers can be freely stacked. Free to use exactly the error checking,
you need. Enable them for debug builds and disable for release.

Vulkan has not validation layers built-in. LunarG SDK bundles many useful layers
(also open source).

Formerly there were instance and device validation layers. Device validation layers
are deprecated now, so all validation layers are used for all devices used in an instance.

All usefull standard validation layers are bundled into a layer in the SDK named
VK_LAYER_KHRONOS_validation. val. layers need to be specified by their name to use them.

A custom message callback for validation layer messages can be created and registered.
For this we need a custom debug messenger.
This needs to be created as an extern function (see ffi).

### debugging instance creation and destruction ###

TODO

### structure extendability ###

TODO

## physical device selection ##

After creating an instance, at least one physical device needs to be selected
for further operations.

## queues and queue families ##

Every command (drawing, uploading textures, etc.) requires a queue, in which the
commands are stored, before they are executed. There are different families of queues,
and every queuefamily only suppports a subset of commands

## logical device ##

- logical devices are required to interface with physical ones
- specify, which queues should be constructed for usage with the logical device

# Window surface #

- vulkan is platform agnostic, therefore can't render to window by itself
	- needs a WSI (Window System integration), e.g. VK_KHR_Surface (instance level extension,
	included in wk_window::get_required_instance_extensions)
	- surface in vulkanalia tutorial is backed by winit
- window surface needs to be created right after instance creation (can influence physical device
  selection)
- window surfaces are optional in vulkan
- surface creation is largely dependent on OS-specifics, which are included in
	vk_window::get_required_instance_extensions and vk_windows::create_surface

## presentation queue

"Presentation is a queue-specific feature, so we need to find the QueueFamily which supports
this feature"
- it could be, that the queue-families, which support drawing, do not overlap
with those which support presentation
