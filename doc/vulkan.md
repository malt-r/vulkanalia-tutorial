# Various bits and notes about vulkan #

## winit ##

The winit::event_loop::EventLoop offers a way to retrieve events from the system.
Calling ::new() will initialize the "context" and everything, that is required for
receiving events.

## memory management ##

All vulkan API calls, which allocate memory (by creating some kind of object),
accept an `allocator` parameter, by which a custom allocator can be passed.
This should probably be used to monitor ressource usage later on.

