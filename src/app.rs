use anyhow::{anyhow, Result};
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    prelude::v1_0::*,
    vk::ExtDebugUtilsExtension,
    window as vk_window,
};
use winit::window::Window;

use std::collections::HashSet;
use log::*;

use crate::{VALIDATION_ENABLED, VALIDATION_LAYER, error::{SuitabilityError, QueueFamilyIndices, self}};

#[derive(Clone, Debug)]
pub struct App {
    // o Entry é próprio do vulkanalia e é quem lida com o carregamento das funções
    entry: Entry,
    // O Instance contém uma instância do Vulkan (vkInstance), pareado a um InstanceCommands, o que encapsula
    // os comandos da instância nela mesma
    instance: Instance,
    // Dados gerais necessários
    data: AppData,
}

impl App {
    pub unsafe fn create(window: &Window) -> Result<Self> {
        // Cria o Loader, que vai carregar o ponteiro das funçẽos do Vulkan
        let loader = LibloadingLoader::new(LIBRARY)?;
        // Entry realmente carrega os erros e tal
        let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;

        let mut data = AppData::default();

        // Instância do Vulkan, necessário pra usar ele
        let instance = App::create_instance(window, &entry, &mut data)?;

        App::pick_physical_device(&instance, &mut data)?;
        Ok(Self {
            entry,
            instance,
            data,
        })
    }

    pub unsafe fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()> {

        for physical_device in instance.enumerate_physical_devices()? {
            let properties = instance.get_physical_device_properties(physical_device);

            if let Err(error) = App::check_physical_device(instance, data, physical_device) {
                warn!("Skipping phyisical device ('{}'): {}", properties.device_name, error);
            } else {
                info!("Selected physical device ('{}').", properties.device_name);
                data.physical_device = physical_device;
                return Ok(());
            }
        }

        Err(anyhow!("Failed to find suitable physical device."))
    }

    pub unsafe fn check_physical_device(instance: &Instance, data: &mut AppData, physical_device: vk::PhysicalDevice) -> Result<()> {
        let properties = instance.get_physical_device_properties(physical_device);
        if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
            return Err(anyhow!(SuitabilityError("Only discrete GPUs supported")));
        }
        
        let features = instance.get_physical_device_features(physical_device);
        if features.geometry_shader != vk::TRUE {
            return Err(anyhow!(SuitabilityError("Missing geometry shader support")));
        }

        QueueFamilyIndices::get(instance, data, physical_device)?;
        Ok(())
    }

    pub unsafe fn render(&self, window: &Window) -> Result<()> {
        Ok(())
    }

    pub unsafe fn destroy(&mut self) {
        if VALIDATION_ENABLED {
            // destruimos nosso logger ...
            self.instance
                .destroy_debug_utils_messenger_ext(self.data.messenger, None);
        }

        // ... E nós mesmos...
        self.instance.destroy_instance(None);
    }

    pub unsafe fn create_instance(
        window: &Window,
        entry: &Entry,
        data: &mut AppData,
    ) -> Result<Instance> {
        // Descreve a aplicação
        let application_info = vk::ApplicationInfo::builder()
            .application_name(b"Vulkan Tutorial\0")
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(b"No Engine\0")
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 0, 0));

        // Extensões necessárias para a execução
        let mut extensions = vk_window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();

        let mut layers: Vec<*const i8> = Vec::new();

        // Se a validação estiver ligada (= modo debug)
        if VALIDATION_ENABLED {
            // Verificamos as layers disponíveis
            let available_layers = entry
                .enumerate_instance_layer_properties()?
                .iter()
                .map(|l| l.layer_name)
                .collect::<HashSet<_>>();

            // Caso não tenha a que queremos (as de validação)
            if !available_layers.contains(&VALIDATION_LAYER) {
                return Err(anyhow!("Validation layer requested but not supported."));
            }

            // Adicionamos as Validation Layers e extensões de debug para melhores erros
            extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
            layers = vec![VALIDATION_LAYER.as_ptr()];
        }

        // Cria a Instância com os parâmetros
        let info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions);

        // Usa o entry, que contém as funções carregadas, pra criar uma instância de Vulkan
        // com as informações que especificamos
        let instance = entry.create_instance(&info, None)?;

        // Caso a validação esteja ligada, adicionamos um logger customizado
        if VALIDATION_ENABLED {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .user_callback(Some(error::debug_callback));

            // Temos que guardar a referência ao logger para destruirmos ele corretamente depois
            data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
        }

        Ok(instance)
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppData {
    messenger: vk::DebugUtilsMessengerEXT,
    physical_device: vk::PhysicalDevice
}