# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Add command buffer lifetime management for buffers and images
- Fix swapchain semaphores indexing issue
- Render gui on first frame
- Enable resizing
- Update to egui 0.32.0

## [0.0.6](https://github.com/n-e-l/cen/compare/v0.0.5...v0.0.6) - 2025-08-08

### Other

- Update release.yml
- Update release.yml
- Support image loading
- Update idk
# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.5](https://github.com/angelocarly/cen/compare/v0.0.4...v0.0.5) - 2025-03-10

### Other

- Update rust.yml
- Update rust.yml
- Update rust.yml
- Update rust.yml
- Update rust.yml
- Remove indices in reverse
- Support single time command buffer finish callbacks
- Add copy image to buffer command
- Correct Linux image format
- Fix Linux build issues
- QOL improvements
- Merge
- Fix warnings
- Fix examples
- Connect gui
- Remove lifetimes and clean up engine
- Add egui and enable dynamic rendering extension
- Construct RenderComponents outside of cen

## [0.0.4](https://github.com/angelocarly/cen/compare/v0.0.3...v0.0.4) - 2024-11-26

### Other

- Add fullscreen option
- Upgrade winit to 0.30.5
- Remove unneeded comment
- Fix examples
- Mutable rendercomponent
- Add update func and use GpuHandle to keep track of cb objects
- Use WindowState instead of Window in the renderer
- Use reference instead of box
- Add very basic buffer bindings
- Add compute example
- Rename basic example to empty
- Make binding call public
- Add binding call to image
- Allow binding multiple push descriptors
- Change logging message from 'kiyo' to 'cen'
- Use PipelineKey instead of DefaultKey

## [0.0.3](https://github.com/angelocarly/cen/compare/v0.0.2...v0.0.3) - 2024-10-19

### Other

- Update README.md
- Update README.md
- Update readme
- Mutable renderer access, shader hotswapping moved to pipeline_store.rs, device to instance memory dependency
- Add basic example
- Remove cpal dependency
- Update README.md
- Remove kiyo specific code

## [0.0.4](https://github.com/angelocarly/kiyo/compare/v0.0.3...v0.0.4) - 2024-08-17

### Other
- Add a little bit of documentation
- Fix compile errors
- Update README.md
- Merge branch 'refs/heads/main' into feature/hot_reload
- Add shader hot-reload
- Fix and improve blur shader
- Improve shader compilation logging
- Clean up examples
- Calculate and pass the macros into the shader compilation
- Pass compute image count through code

## [0.0.3](https://github.com/angelocarly/kiyo/compare/v0.0.2...v0.0.3) - 2024-08-14

### Other
- Create release.yml
- Switch swapchain image copy to a blit
- Lighten the blur pass
- Merge pull request [#19](https://github.com/angelocarly/kiyo/pull/19) from angelocarly/feature/fps_counter
- Add fps logging and vsync option
- Remove calloop log spam
- Automatically deduce the image count
- Add logging
