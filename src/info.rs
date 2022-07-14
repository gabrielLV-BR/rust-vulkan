use std::collections::HashSet;

use anyhow::{anyhow, Result};
use vulkanalia::{
    vk::{
        self, DeviceV1_0, Handle, HasBuilder, Image, InstanceV1_0, KhrSurfaceExtension,
        KhrSwapchainExtension,
    },
    Device, Instance,
};
use winit::window::Window;

use crate::error;
use crate::{app::AppData, error::SuitabilityError, DEVICE_EXTENSIONS};

#[derive(Copy, Clone, Debug)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    pub present: u32,
}

impl QueueFamilyIndices {
    pub unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties = instance.get_physical_device_queue_family_properties(physical_device);

        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        let mut present = None;
        for (index, properties) in properties.iter().enumerate() {
            if instance.get_physical_device_surface_support_khr(
                physical_device,
                index as u32,
                data.surface,
            )? {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            Err(anyhow!(error::SuitabilityError(
                "Missing required queue families"
            )))
        }
    }
}

#[derive(Clone, Debug)]
pub struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapchainSupport {
    pub unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        Ok(Self {
            capabilities: instance
                .get_physical_device_surface_capabilities_khr(physical_device, data.surface)?,
            formats: instance
                .get_physical_device_surface_formats_khr(physical_device, data.surface)?,
            present_modes: instance
                .get_physical_device_surface_present_modes_khr(physical_device, data.surface)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct SwapchainData {
    pub chain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub image_views: Vec<vk::ImageView>,
}

impl SwapchainData {
    pub unsafe fn create_swapchain(
        window: &Window,
        instance: &Instance,
        device: &Device,
        data: &AppData,
    ) -> Result<Self> {
        let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
        let support = SwapchainSupport::get(instance, data, data.physical_device)?;

        // Formato da Swapchain: Modo de canal de cores e colorspace
        let surface_format = Self::get_swapchain_surface_format(&support.formats);
        // Present mode: V-buffer, triple buffer...
        let present_mode = Self::get_swapchain_present_mode(&support.present_modes);
        // Extent: Tamanho da imagem (surface onde vamos desenhar)
        let extent = Self::get_swapchain_extent(window, support.capabilities);

        let mut image_count = support.capabilities.min_image_count + 1;

        if support.capabilities.max_image_count != 0
            && image_count > support.capabilities.max_image_count
        {
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

        let chain = device.create_swapchain_khr(&info, None)?;
        let images = device.get_swapchain_images_khr(chain)?;
        let format = surface_format.format;
        let image_views = Self::create_swapchain_image_views(device, &images, &format)?;

        Ok(Self {
            chain,
            extent,
            format,
            images,
            image_views,
        })
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_swapchain_khr(self.chain, None);
    }

    pub unsafe fn create_swapchain_image_views(
        device: &Device,
        images: &Vec<Image>,
        format: &vk::Format,
    ) -> Result<Vec<vk::ImageView>> {
        let components = vk::ComponentMapping::builder()
            .r(vk::ComponentSwizzle::IDENTITY)
            .g(vk::ComponentSwizzle::IDENTITY)
            .b(vk::ComponentSwizzle::IDENTITY)
            .a(vk::ComponentSwizzle::IDENTITY);

        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(0);

        let data = images
            .iter()
            .map(|i| {
                let info = vk::ImageViewCreateInfo::builder()
                    .image(*i)
                    .view_type(vk::ImageViewType::_2D)
                    .format(*format)
                    .components(components)
                    .subresource_range(subresource_range)
                    .build();

                device.create_image_view(&info, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(data)
    }

    pub unsafe fn check_physical_device_extensions(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Result<()> {
        let extensions = instance
            .enumerate_device_extension_properties(physical_device, None)?
            .iter()
            .map(|e| e.extension_name)
            .collect::<HashSet<_>>();

        if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
            return Ok(());
        }
        Err(anyhow!(SuitabilityError(
            "Device does not have required extensions"
        )))
    }

    pub unsafe fn get_swapchain_surface_format(
        formats: &[vk::SurfaceFormatKHR],
    ) -> vk::SurfaceFormatKHR {
        formats
            .iter()
            .cloned()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| formats[0])
    }

    pub unsafe fn get_swapchain_present_mode(
        present_modes: &[vk::PresentModeKHR],
    ) -> vk::PresentModeKHR {
        present_modes
            .iter()
            .cloned()
            .find(|f| *f == vk::PresentModeKHR::MAILBOX)
            .unwrap_or_else(|| vk::PresentModeKHR::FIFO)
    }

    pub unsafe fn get_swapchain_extent(
        window: &Window,
        capabilites: vk::SurfaceCapabilitiesKHR,
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
                    size.width,
                ))
                .height(clamp(
                    capabilites.min_image_extent.height,
                    capabilites.max_image_extent.height,
                    size.height,
                ))
                .build()
        }
    }
}
