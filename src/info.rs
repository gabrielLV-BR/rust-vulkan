use vulkanalia::{vk::{self, InstanceV1_0, KhrSurfaceExtension}, Instance};
use anyhow::{anyhow, Result};

use crate::app::AppData;
use crate::error;

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
            if instance.get_physical_device_surface_support_khr(physical_device, index as u32, data.surface)? {
                present = Some(index as u32);
                break;
            } 
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok(Self { graphics, present })
        } else {
            Err(anyhow!(error::SuitabilityError("Missing required queue families")))
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
    physical_device: vk::PhysicalDevice
  ) -> Result<Self> {
    Ok(
      Self {
        capabilities: instance.get_physical_device_surface_capabilities_khr(physical_device, data.surface)?,
        formats: instance.get_physical_device_surface_formats_khr(physical_device, data.surface)?,
        present_modes: instance.get_physical_device_surface_present_modes_khr(physical_device, data.surface)?
      }
    )
  }
}