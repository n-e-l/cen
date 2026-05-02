use std::collections::HashMap;
use std::{fmt, fs};
use std::path::PathBuf;
use ash::vk;
use ash::vk::ShaderModule;
use log::{info, trace};
use shaderc::{IncludeType, ResolvedInclude};
use crate::vulkan::{LOG_TARGET};
use crate::vulkan::memory::GpuResource;

pub trait Pipeline {
    fn handle(&self) -> vk::Pipeline;
    fn bind_point(&self) -> vk::PipelineBindPoint;
    fn layout(&self) -> vk::PipelineLayout;
    fn resource(&self) -> &dyn GpuResource;
}

pub fn create_shader_module(device: &ash::Device, code: Vec<u32>) -> ShaderModule {
    let shader_module_create_info = vk::ShaderModuleCreateInfo::default()
        .code(unsafe { std::slice::from_raw_parts(code.as_ptr(), code.len()) });

    unsafe {
        device
            .create_shader_module(&shader_module_create_info, None)
            .expect("Failed to create shader module")
    }
}

#[derive(Debug)]
pub enum PipelineErr {
    ShaderCompilation(String)
}

impl fmt::Display for PipelineErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PipelineErr::ShaderCompilation(ref err) => {
                write!(f, "{}", err)
            },
        }
    }
}

#[derive(Clone)]
pub struct SlangModule {
    pub name: String,
    pub source: String,
}

pub fn load_slang_shader_code(source_file: PathBuf, modules: &[SlangModule]) -> Result<Vec<u32>, PipelineErr> {
    use shader_slang as slang;

    let global_session = slang::GlobalSession::new()
        .ok_or_else(|| PipelineErr::ShaderCompilation("Failed to create Slang global session".into()))?;

    let target_desc = slang::TargetDesc::default()
        .format(slang::CompileTarget::Spirv)
        .profile(global_session.find_profile("glsl_450"));

    let targets = [target_desc];
    let session_desc = slang::SessionDesc::default().targets(&targets);

    let session = global_session.create_session(&session_desc)
        .ok_or_else(|| PipelineErr::ShaderCompilation("Failed to create Slang session".into()))?;

    // Pre-load user modules so the host shader can import them by name
    let user_components: Vec<slang::ComponentType> = modules.iter()
        .map(|m| {
            session
                .load_module_from_source_string(&m.name, &format!("{}.slang", m.name), &m.source)
                .map(|module| module.into())
                .map_err(|e| PipelineErr::ShaderCompilation(format!("Module '{}': {}", m.name, e)))
        })
        .collect::<Result<_, _>>()?;

    let source = fs::read_to_string(&source_file)
        .map_err(|e| PipelineErr::ShaderCompilation(format!("{:?}: {}", source_file, e)))?;
    let module_name = source_file.file_stem().unwrap_or_default().to_string_lossy();
    let path = source_file.to_str().unwrap();

    let host_module = session
        .load_module_from_source_string(&module_name, path, &source)
        .map_err(|e| PipelineErr::ShaderCompilation(format!("{}", e)))?;

    let entry_point = host_module
        .find_entry_point_by_name("main")
        .ok_or_else(|| PipelineErr::ShaderCompilation("Entry point 'main' not found".into()))?;

    let mut components: Vec<slang::ComponentType> = vec![
        host_module.into(),
        entry_point.into(),
    ];
    components.extend(user_components);

    let linked = session
        .create_composite_component_type(&components)
        .map_err(|e| PipelineErr::ShaderCompilation(format!("{}", e)))?
        .link()
        .map_err(|e| PipelineErr::ShaderCompilation(format!("{}", e)))?;

    let bytes = linked
        .entry_point_code(0, 0)
        .map_err(|e| PipelineErr::ShaderCompilation(format!("{}", e)))?;

    let bytes = bytes.as_slice();
    if bytes.len() % 4 != 0 {
        return Err(PipelineErr::ShaderCompilation("Invalid SPIR-V output".into()));
    }

    let spirv = bytes.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();

    trace!(target: LOG_TARGET, "Compiled Slang shader: {:?}", source_file);
    Ok(spirv)
}

/**
 * Load a shader from a file and compile it into SPIR-V.
 */
pub fn load_shader_code(source_file: PathBuf, macros: &HashMap<String, String>) -> Result<Vec<u32>, PipelineErr>
{
    use shaderc;

    let shader_kind = match source_file.to_str().unwrap().split(".").last() {
        Some("vert") => shaderc::ShaderKind::Vertex,
        Some("frag") => shaderc::ShaderKind::Fragment,
        Some("comp") => shaderc::ShaderKind::Compute,
        _ => panic!("Unknown shader type")
    };

    let source = fs::read_to_string(source_file.clone()).unwrap_or_else(|_| panic!("Failed to read file: {:?}", source_file));

    let compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_include_callback(|include_name, include_type, original_source, _| {
        let original_path = PathBuf::from(original_source);

        match include_type {
            IncludeType::Relative => {
                let path = original_path.parent().unwrap().join(PathBuf::from(include_name));
                let source = fs::read_to_string(path.clone()).unwrap_or_else(|_| panic!("Failed to read file: {:?}", path));
                info!("Loaded shader include: {}", path.to_str().unwrap());
                Ok(ResolvedInclude {
                    resolved_name: path.to_str().unwrap().to_string(),
                    content: source,
                })
            }
            IncludeType::Standard => {
                Err(format!("Only relative includes are supported. Can't include {}", include_name))
            }
        }

    });
    options.add_macro_definition("EP", Some("main"));
    for ( k, v ) in macros {
        options.add_macro_definition(k, Some(v.to_string().as_str()));
    }

    let binary_result = compiler.compile_into_spirv(
        source.as_str(),
        shader_kind,
        source_file.to_str().unwrap(),
        "main",
        Some(&options)
    );

    match binary_result {
        Ok(result) => {
            trace!(target: LOG_TARGET, "Compiled shader code: {:?}", source_file);
            Ok(result.as_binary().to_vec())
        },
        Err(error) => {
            Err(PipelineErr::ShaderCompilation(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use ash::Entry;
    use super::*;
    use crate::vulkan::Instance;

    const SPIRV_MAGIC: u32 = 0x07230203;

    #[test]
    fn slang_compiles_to_valid_spirv() {
        let spirv = load_slang_shader_code("examples/slang/shader.slang".into(), &[])
            .expect("Slang compilation failed");

        assert!(!spirv.is_empty(), "SPIR-V output is empty");
        assert_eq!(spirv[0], SPIRV_MAGIC, "Missing SPIR-V magic number");
    }

    #[test]
    fn slang_shader_accepted_by_vulkan() {
        let entry = Entry::linked();
        let instance = Instance::new(&entry, None);
        let (physical_device, queue_family_index) = instance.create_physical_device_headless();
        let device = crate::vulkan::Device::new(&instance, physical_device, queue_family_index);

        let spirv = load_slang_shader_code("examples/slang/shader.slang".into(), &[])
            .expect("Slang compilation failed");

        let module = create_shader_module(device.handle(), spirv);
        unsafe { device.handle().destroy_shader_module(module, None) };
    }

    #[test]
    fn slang_with_user_module() {
        let user_module = SlangModule {
            name: "palette".into(),
            source: "public float3 custom_palette(float t) { return float3(t, 1.0 - t, 0.5); }".into(),
        };

        let spirv = load_slang_shader_code("examples/slang/shader.slang".into(), &[user_module])
            .expect("Slang compilation with user module failed");

        assert!(!spirv.is_empty());
        assert_eq!(spirv[0], SPIRV_MAGIC);
    }
}
