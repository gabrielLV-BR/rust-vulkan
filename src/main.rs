#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

mod error;
mod app;
mod info;

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;
use winit::{event_loop::{EventLoop, ControlFlow}, window::WindowBuilder, dpi::LogicalSize, event::{WindowEvent, Event}};

const VALIDATION_ENABLED: bool = true /* cfg!(debug_assertions) */;
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

fn main() -> Result<()> {
    // Queremos logs bonitos
    pretty_env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Learning Vulkan (Oh boy)")
        .with_inner_size(LogicalSize::new(600, 600))
        .build(&event_loop)?;

    let mut app = unsafe { app::App::create(&window)? };
    let mut destroying = false;

    // Janela básica do winit
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared if !destroying => unsafe {
                app.render(&window).unwrap();
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                log::warn!("VAI TOAMR NO CU");
                destroying = true;
                *control_flow = ControlFlow::Exit;
                unsafe {
                    app.destroy();
                }
            }
            _ => {}
        }
    });
}
