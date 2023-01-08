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

## swapchains ##

- There is not default framebuffer
- We need some object to own the buffers to render to -> swapchain
- basically a queue of images to render and draw to and is used for synchronizing
presentation and rendering with refreshing of the screen
- not all devices support rendering (server GPUs), so swapchain support is implemented
in an extension (vulkanalia::vk::KhrSwapchainExtension)
- availability of a presentation queue implies support of swapchains

### details of swapchain support ###

- just checking, if swapchains are supported is not sufficient
- Also needed:
	- basic surface capabilities (min/max number of images in swapchain, min/max/ width & height)
	- surface formats (pixel format, color space)
	- available presentation modes

Choosing right settings for the swapchain
- surface format (color depth)
- presentation mode (conditions for swapping images to the screen)
- swap extent (resolution of images in swapchain)

#### presentation modes ####

Immediate:
- images submitted by app are transfered to screen immediately -> may cause vertical tearing

FIFO:
- store fixed amount of images in a queue, if queue is full, app needs to wait
- screen refreshing moment also known as "vertical blank"
- is the only mode, which is guaranteed to be supported

FIFO-relaxed:
- like FIFO, but if the app is late, presentation won't wait for the next vertical
blank to transfer the image to the screen but will draw it immediately on arrival
(may cause tearing)

Mailbox:
- like FIFO, but app won't wait on full queue, but instead render images as fast as
possible and swap last image with the latest one (also known as 'triple buffering')
- draw frame as fast as possible without tearing and reduced latency
- consumes more energy than FIFO

#### swap extent ####

- resolution of swapchain images (almost always exactly equal to resolution of window
we are drawing to)
- some window manager allow for deviation of this (indicated by `current_extent` set
to max value of u32)

# Order of operations

## initial setup

- create window (with external library)
- initialize vulkan instance 
	- accesspoint for all other vulkan operations
	- uses window to load required vulkan extensions
	- setup debug utilities (validation layers) and debug callback
- create window surface (abstraction layer between app and os-specific window 
	access)

## create logical device

- pick a physical device, which conforms with required device properties 
	(check DEVICE_EXTENSIONS)
- create logical device (abstraction layer between app and physical device)
	- specify, which validation layers, extensions and features are enabled in 
		the logical device
	- provides access to graphics_queue and present_queue (for presentation)
	
## create swapchain

- swapchain contains the images, to which should be rendered in order to 
	draw to the window surface
- specify color space and format (how many bits for each color, etc.), presentation 
	mode and extent (roughly speaking: image size)
- specify which queue families (of the physical device) should be used
- store the swapchain images in appdata after creation

## create render pass

- a render pass describes the data structures (attachments) and subpasses (which 
	will use the attachments) and dependencies between subpasses
- specify load and store operations for attachments (in AttachmentDescription)

## create pipeline

- create vertex and fragment shader modules and add them to stages
- specify viewport and scissor dimensions
- configure rasterizer
- configure multisampling
- specify color blend method and add it as attachement to the color blend stage
- create pipeline object
- destroy shader modules (no longer needed, if no extensive shader debuggin is 
	needed)
	
## create swapchain image views

- create a view into the images of the swapchain (needed in order to use the 
	swapchain image)
- specify, what the views purpose is (in subresource_range)

## create framebuffers

- bind the swapchain image views to an attachment (defined during render pass 
	creation) by creating a framebuffer object
	- references the render pass and the image view (as an attachement)
	
## create command pool

- create command pool(s), which are used to allocate command buffers
- specify the type of queue (by queue family index) to which the command buffers 
	created from this pool will be submitted

## create vertex buffer

TODO

- memory, that is used to upload vertex data from the CPU may not be the most optimal 
	memory for the GPU to read from (that would be memory, which has the 
	vk::MemoryPropertyFlags::DEVICE_LOCAL flag, but that is not accessible by the CPU
	for writes, if the GPU is a dedicated device)
- solution: create two vertex buffers, one staging buffer, which will be written to
	by the CPU and the final vertex buffer in device local memory
- this requires a buffer copy command, which in turn requires a queue family, which 
	supports transfer operations (vk::QueueFlags::TRANSFER) - any queue with vk::QueueFlags::GRAPHICS or ::COMPUTE
	implicityly supports ::TRANSFER operations
	- this could be a chance to practice using different queue families (one specifically for 
		transfer operations)
	
## create command buffer(s)

- allocate a command buffer for each framebuffer (each framebuffer binds one 
	swapchain image as an attachement)
- begin recording of command buffer
- begin render pass
	- reference renderpass
	- reference framebuffer
	- specify render area
	- specify clear values (the color, which should be used for clearing all pixels)
- bind pipeline (tells vulkan, which attachements to use(?))
- add draw command (currently the vertices are hard coded into the vertex shader)
- end render pass
- end command buffer

## create synchronization objects

- create semaphores; synchronization of rendering itself (for each image/frame in flight)
	- image-ready semaphore
	- render-finished semaphore
- create fences; synchronization of rendering with app (how are these different?)
	- in_flight_fences (for each image/frame in flight)
	- images_in_flight (for each image/frame in flight) (initialized with fence::null)
	- acquire_next_image might return swapchain images out of order from frames;
		

## render 

- wait for fence of current frame
- acquire next image (vulkan call) -> returns image index (index into the swapchain images)
	- pass image-ready semaphore for the current frame
- store fence from `in_flight_fences[frame_idx]` at the `images_in_flight[image_index]` 
	- aquire_next_image_khr might return images out of order from the images_in_flight 
		order and might return a frame_index of an image, which is already rendered at
	- in order to avoid rendering to an image, which is already rendered at, we store 
		the fence from in_flight_fences in the helper array images_in_flight (which 
		stores the fences of the images !CURRENTLY IN FLIGHT!)
- reset the fence of the frame
- submit command buffer
	- command buffer is tied to the swapchain image, therefore we need the corresponding 
		command buffer for the `image_index`
	- pass semaphore for which to wait
	- pass the stage in which to wait for the wait-semaphore
	- pass semaphore which to signal, when rendering is complete
- present rendered image
	- pass signal_semaphore
	- pass reference to swapchain
	- pass image_indices

## indexed rendering with vertex buffers

From tutorial:
The previous chapter already mentioned that you should allocate multiple resources
like buffers from a single memory allocation, but in fact you should go a step 
further. Driver developers recommend that you also store multiple buffers, like
the vertex and index buffer, into a single vk::Buffer and use offsets in commands
like cmd_bind_vertex_buffers. The advantage is that your data is more cache friendly
in that case, because it's closer together. It is even possible to reuse the same 
chunk of memory for multiple resources if they are not used during the same render
operations, provided that their data is refreshed, of course. This is known as 
aliasing and some Vulkan functions have explicit flags to specify that you want
to do this.

## image layout transitions

- Images must be stored in different layouts for different operations (Transfer_DST_Optimal, Shader_Read_Only_Optimal, etc.)
- One of the most common ways to do layout transitions is image memory barrier 
	(which is a pipeline barrier)
	- pipeline barrier like that is used to synchronize access to resources

## pipeline barriers

see:  https://gpuopen.com/learn/vulkan-barriers-explained/

GPU is a highly pipelined device. Commands come in at top, go through stages and 
end up at bottom.

"Source" and "Target" Stages in context of a barrier: Producer and Consumer stages of 
the pipeline. By specifying these, we tell the driver, what operations need to 
finish before the "transition" can execute and what must not have been started (target).

Example from website:
"Imagine you have a vertex shader that also stores data via an imageStore and a 
compute shader that wants to consume it. In this case you wouldn’t want to wait
for a subsequent fragment shader to finish as this can take a long time to complete.
You really want the compute shader to start as soon as the vertex shader is done. 
The way to express this is to set the source stage -the producer- to VERTEX_SHADER_BIT
and the target stage -the consumer- to COMPUTE_SHADER_BIT "

## access mask and pipeline stage flags 

see: https://www.reddit.com/r/vulkan/comments/muo5ud/comment/gv8kzxi/?utm_source=share&utm_medium=web2x&context=3

Stage flags specify execution order (which operations must be performed before 
and can start after).

The AccessMasks are related to memory/caching and specify, how resources are accessed 
by an operation. Even though SubPass A (source) writes to a memory location and B (target)
reads from it, does not mean, that B "sees" the changes a made (because of caching).
Therefore we should specify, which operations are performed by A and B, so that 
the driver knows, that the changes made by A need to be flushed to main memory and 
not just stored in the cache.
