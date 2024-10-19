# Cen
![build](https://github.com/angelocarly/cen/actions/workflows/rust.yml/badge.svg)
[![crate](https://img.shields.io/crates/v/cen)](https://crates.io/crates/cen/)  

A lightweight vulkan window backend using [ash](https://github.com/ash-rs/ash).

Cen aims to provide abstractions around Vulkan handles and window management. Current features include:
- Winit-backed window setup
- Vulkan wrappers with memory dependencies
- Hot-swappable shader storage, compiled at runtime

## Building & running

Make sure you have the [Vulkan SDK](https://vulkan.lunarg.com) installed.  
Then build and run `cen` examples:
```
git clone https://github.com/angelocarly/cen.git
cd cen
cargo run --example basic
```

## GPU debugging

### Windows & Linux
Renderdoc!

### Mac
Mac only has XCode's Metal debugger. In order to use it you need to provide the following environment variables:
```bash
VULKAN_SDK=$HOME/VulkanSDK/<version>/macOS
DYLD_FALLBACK_LIBRARY_PATH=$VULKAN_SDK/lib
VK_ICD_FILENAMES=$VULKAN_SDK/share/vulkan/icd.d/MoltenVK_icd.json
VK_LAYER_PATH=$VULKAN_SDK/share/vulkan/explicit_layer.d
```

Then you should be able to launch your cen application and capture a frame.  
[This video](https://www.youtube.com/watch?v=uNB4RMZg1AM) does a nice job explaining the process.

## References
- [myndgera](https://github.com/pudnax/myndgera) - Pipeline caching and reloading
- [paya](https://github.com/paratym/paya) - Vulkan memory dependencies and ash wrappers

## Libraries
- [ash](https://github.com/ash-rs/ash) - Vulkan bindings
- [winit](https://github.com/rust-windowing/winit) - Window creation and handling
- [shaderc](https://github.com/google/shaderc-rs) - Shader compilation
- [gpu-allocator](https://github.com/Traverse-Research/gpu-allocator?tab=readme-ov-file) - Memory management
- [notify](https://github.com/notify-rs/notify) - File watching
