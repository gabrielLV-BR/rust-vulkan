#![allow(
    dead_code,
    unused_variables,
    unused_imports,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod utils;

use anyhow::{anyhow, Result};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder}
};
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    window as vk_window,
    prelude::v1_0::*,
    vk::ExtDebugUtilsExtension
};

use std::{
    collections::HashSet,
    ffi::CStr,
    os::raw::c_void
};

use log::*;

const VALIDATION_ENABLED: bool = true /* cfg!(debug_assertions) */;
const VALIDATION_LAYER: vk::ExtensionName = 
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

fn main() -> Result<()> {

    pretty_env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Learning Vulkan (Oh boy)")
        .with_inner_size(LogicalSize::new(600, 600))
        .build(&event_loop)?;

    let mut app = unsafe { App::create(&window)? };
    let mut destroying = false;

    event_loop.run(move 
        | event, _, 
          control_flow | 
          {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::MainEventsCleared if !destroying => {
                    unsafe { app.render(&window).unwrap(); }
                },
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    destroying = true;
                    *control_flow = ControlFlow::Exit;
                    unsafe { app.destroy(); }
                },
                _ => {}
            }
    });
}

#[derive(Clone, Debug)]
struct App {
    // o Entry é próprio do vulkanalia e é quem lida com o carregamento dos CUS
    entry: Entry,
    instance: Instance,
    data: AppData
}

impl App {
    unsafe fn create(window: &Window) -> Result<Self> { 
        // Cria o Loader, que vai carregar o ponteiro das funçẽos do Vulkan
        let loader = LibloadingLoader::new(LIBRARY)?;
        // 
        let entry = Entry::new(loader)
            .map_err(|b| anyhow!("{}", b))?;

        let mut data = AppData::default();

        let instance = create_instance(window, &entry, &mut data)?;

        Ok( Self { entry, instance, data } )
        // Ok(App {}) 
    }

    unsafe fn render(&self, window: &Window) -> Result<()> { Ok(())}

    unsafe fn destroy(&mut self) {

        if VALIDATION_ENABLED {
            self.instance.destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        self.instance.destroy_instance(None);
    }
}
    unsafe fn create_instance(
        window: &Window, entry: &Entry, data: &mut AppData
    ) -> Result<Instance> {
        // Descreve a aplicação
        let application_info = vk::ApplicationInfo::builder()
            .application_name(b"Vulkan Tutorial\0")
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(b"No Engine\0")
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 0, 0));
        
        // Extensões necessárias para a execução
        let mut extensions = 
            vk_window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();

        if VALIDATION_ENABLED {
            extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
        }

        let available_layers = entry
            .enumerate_instance_layer_properties()?
            .iter()
            .map(|l| l.layer_name)
            .collect::<HashSet<_>>();

        if VALIDATION_ENABLED &&
            !available_layers.contains(&VALIDATION_LAYER) {
            return Err(anyhow!("Validation layer requested but not supported."));
        }

        let layers = if VALIDATION_ENABLED {
            vec![VALIDATION_LAYER.as_ptr()]
        } else {
            Vec::new()
        };

        // Cria a Instância com os parâmetros
        let info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions);

        let instance = entry.create_instance(&info, None)?;

        if VALIDATION_ENABLED {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .user_callback(Some(utils::debug_callback));

            data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
        }

        Ok(instance)
    }


#[derive(Clone, Debug, Default)]
struct AppData {
    messenger: vk::DebugUtilsMessengerEXT,
}