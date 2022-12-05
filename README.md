This is a custom Vulkan abstraction layer I built that is used internally by ``cflake-engine``.

This Vulkan Abstraction Layer allows me to hide the painstaking and ugly parts of vulkan (boilerplate and manual synchronization) and handle them automatically for us.
This should be considered a safe wrapper around raw Vulkan (Ash). 

This crate contains a ``Recorder`` struct that automatically records commands and saves them internally, then it will manually put synchronization barriers and pipeline barriers automatically when needed. A sample program that uses the recorder could look like this

```rs
// Fetch a new recorder from the queue
let recorder: Recorder = queue.acquire();

// Set commands
recorder.cmd_clear_image(todo!());

// Submit the recorder TO THE SAME QUEUE
let id = queue.submit(recorder);
id.wait();
``` 

Please help me Vulkan is killing me