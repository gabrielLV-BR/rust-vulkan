use anyhow::{anyhow, Result};
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY},
    prelude::v1_0::*,
    vk::{ExtDebugUtilsExtension, KhrSurfaceExtension, KhrSwapchainExtension},
    window as vk_window,
};
use winit::window::Window;

use std::collections::HashSet;
use log::*;

use crate::{VALIDATION_ENABLED, VALIDATION_LAYER, 
    error::{SuitabilityError, self}, DEVICE_EXTENSIONS,
    info::{QueueFamilyIndices, SwapchainSupport}};

#[derive(Clone, Debug)]
pub struct App {
    // o Entry é próprio do vulkanalia e é quem lida com o carregamento das funções
    entry: Entry,
    // O Instance contém uma instância do Vulkan (vkInstance), pareado a um InstanceCommands, o que encapsula
    // os comandos da instância nela mesma
    instance: Instance,
    // Dados gerais necessários
    data: AppData,
    // Referência lógica ao dispositivo (GPU)
    device: Device,
}

impl App {
    pub unsafe fn create(window: &Window) -> Result<Self> {
        // Cria o Loader, que vai carregar o ponteiro das funçẽos do Vulkan
        let loader = LibloadingLoader::new(LIBRARY)?;
        // Entry realmente carrega os erros e tal
        let entry = Entry::new(loader)
            .map_err(
                |b| anyhow!("{}", b)
            )?;

        let mut data = AppData::default();

        // Instância do Vulkan, necessário pra usar ele
        let instance = App::create_instance(window, &entry, &mut data)?;
        data.surface = vk_window::create_surface(&instance, window)?;
        App::pick_physical_device(&instance, &mut data)?;

        let device = App::create_logical_device(&instance, &mut data)?;

        App::create_swapchain(window, &instance, &device, &mut data)?;

        Ok(Self {
            entry,
            instance,
            data,
            device
        })
    }

    unsafe fn create_logical_device(instance: &Instance, data: &mut AppData) 
        -> Result<Device> {
        let indices = 
            QueueFamilyIndices::get(instance, data, data.physical_device)?;

        let mut unique_indices = HashSet::new();
        unique_indices.insert(indices.graphics);
        unique_indices.insert(indices.present);

        let queue_priorities = &[1.0];
        let queue_info = unique_indices
            .iter()
            .map(|i| {
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(*i)
                    .queue_priorities(queue_priorities)
            }).collect::<Vec<_>>();

        // Layers específicas ao dispositivo.
        // Em Vulkan moderno  as layers do escopo da instância, isso é pra 
        // ser compatível com versões anteriores que permitiam extensões
        // especificas para cada dispositivo virtual
        let layers = if VALIDATION_ENABLED {
            vec![VALIDATION_LAYER.as_ptr()]
        } else { vec![] };

        // Recursos do dispositivo (o qual verificamos a existência no check_physical_device())
        let features = vk::PhysicalDeviceFeatures::builder();

        let extensions = DEVICE_EXTENSIONS
            .iter()
            .map(|n| n.as_ptr())
            .collect::<Vec<_>>();

        let info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&features);

        let device = 
            instance.create_device(data.physical_device, &info, None)?;

        data.present_queue = device.get_device_queue(indices.present, 0);
        data.graphics_queue = device.get_device_queue(indices.graphics, 0);

        Ok(device)
    }

    unsafe fn pick_physical_device(instance: &Instance, data: &mut AppData) 
        -> Result<()> {

        for physical_device in instance.enumerate_physical_devices()? {
            let properties = 
                instance.get_physical_device_properties(physical_device);

            if let Err(error) = 
                App::check_physical_device(instance, data, physical_device) {
                warn!("Skipping phyisical device ('{}'): {}", properties.device_name, error);
            } else {
                info!("Selected physical device ('{}').", properties.device_name);
                data.physical_device = physical_device;
                return Ok(());
            }
        }

        Err(anyhow!("Failed to find suitable physical device."))
    }

    unsafe fn check_physical_device(
            instance: &Instance, 
            data: &mut AppData, 
            physical_device: vk::PhysicalDevice
        ) -> Result<()> {
        let properties = 
            instance.get_physical_device_properties(physical_device);
        if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
            return Err(anyhow!(SuitabilityError("Only discrete GPUs supported")));
        }
        
        let features = 
            instance.get_physical_device_features(physical_device);
        if features.geometry_shader != vk::TRUE {
            return Err(anyhow!(SuitabilityError("Missing geometry shader support")));
        }

        QueueFamilyIndices::get(instance, data, physical_device)?;

        App::check_physical_device_extensions(instance, physical_device)?;

        let support = SwapchainSupport::get(instance, data, physical_device)?;
        if support.formats.is_empty() || support.present_modes.is_empty() {
            return Err(anyhow!(SuitabilityError("Insuficient swapchain support")));
        }

        Ok(())
    }

    pub unsafe fn create_swapchain(
        window: &Window, instance: &Instance,
        device: &Device, data: &mut AppData
    ) -> Result<()> {

        let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
        let support = SwapchainSupport::get(instance, data, data.physical_device)?;

        // Formato da Swapchain: Modo de canal de cores e colorspace
        let surface_format = App::get_swapchain_surface_format(&support.formats);
        // Present mode: V-buffer, triple buffer...
        let present_mode = App::get_swapchain_present_mode(&support.present_modes);
        // Extent: Tamanho da imagem (surface onde vamos desenhar)
        let extent = App::get_swapchain_extent(window, support.capabilities);

        let mut image_count = support.capabilities.min_image_count + 1;

        if support.capabilities.max_image_count != 0 &&
            image_count > support.capabilities.max_image_count {
            image_count = support.capabilities.max_image_count; 
        }

        let mut queue_family_indices = vec![];
        let image_sharing_mode = if indices.graphics != indices.present {
            queue_family_indices.push(indices.graphics);
            queue_family_indices.push(indices.present);
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };

        // Um monstro que descreve exatamente como queremos nossa swapchain
        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(data.surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        data.swapchain = device.create_swapchain_khr(&info, None)?;    

        Ok(())
    }

    pub unsafe fn check_physical_device_extensions(
        instance: &Instance,
        physical_device: vk::PhysicalDevice
    ) -> Result<()> {
        let extensions = instance
            .enumerate_device_extension_properties(physical_device, None)?
            .iter()
            .map(|e| e.extension_name)
            .collect::<HashSet<_>>();

        if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
            return Ok(());
        }
        Err(anyhow!(SuitabilityError("Device does not have required extensions")))
    }

    pub unsafe fn get_swapchain_surface_format(
        formats: &[vk::SurfaceFormatKHR]
    ) -> vk::SurfaceFormatKHR {
        formats
            .iter()
            .cloned()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB &&
                f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            }).unwrap_or_else(|| formats[0])
    }

    pub unsafe fn get_swapchain_present_mode(
        present_modes: &[vk::PresentModeKHR]
    ) -> vk::PresentModeKHR {
        present_modes
            .iter()
            .cloned()
            .find(|f| {
                *f == vk::PresentModeKHR::MAILBOX
            }).unwrap_or_else(|| vk::PresentModeKHR::FIFO)
    }

    pub unsafe fn get_swapchain_extent(
        window: &Window,
        capabilites: vk::SurfaceCapabilitiesKHR
    ) -> vk::Extent2D {
        if capabilites.current_extent.width != u32::MAX {
            capabilites.current_extent
        } else {
            let size = window.inner_size();
            let clamp = |min: u32, max: u32, v: u32| min.max(max.min(v));
            vk::Extent2D::builder()
            .width(clamp(
                capabilites.min_image_extent.width,
                capabilites.max_image_extent.width,
                size.width
            ))
            .height(clamp(
                capabilites.min_image_extent.height,
                capabilites.max_image_extent.height,
                size.height
            )).build()
        }
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

        // ... Nossa swapchain...
        self.device.destroy_swapchain_khr(self.data.swapchain, None);
        // ... Nosso dispositivo virtual...
        self.device.destroy_device(None);
        // ... Nosso Surface...
        self.instance.destroy_surface_khr(self.data.surface, None);
        // ... E nós mesmos...
        self.instance.destroy_instance(None);
    }

    pub unsafe fn create_instance(
        window: &Window,
        entry: &Entry,
        data: &mut AppData,
    ) -> Result<Instance> {
        // Descreve a aplicação
        let application_info = 
            vk::ApplicationInfo::builder()
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
            let debug_info = 
                vk::DebugUtilsMessengerCreateInfoEXT::builder()
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
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue: vk::Queue,
    pub surface: vk::SurfaceKHR,
    pub present_queue: vk::Queue,
    pub swapchain: vk::SwapchainKHR
}