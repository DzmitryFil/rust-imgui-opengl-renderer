use glow;
use glow::HasContext;
use imgui::*;
use imgui_opengl;
use imgui_winit_support;
use std::time::Instant;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow},
    dpi::PhysicalSize,
};

fn main() {
    env_logger::init();

    let (mut gl, window, event_loop, resize_func, swap_func) = {
        let el = glutin::event_loop::EventLoop::new();
        let wb = glutin::window::WindowBuilder::new()
            .with_title("Hello")
            .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
        let windowed_context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGlEs, (2, 0)))
            .build_windowed(wb, &el)
            .unwrap();

        let windowed_context = unsafe { windowed_context.make_current().unwrap() };

        let (context_wrapper, window) = unsafe { windowed_context.split() };

        let context_wrapper = std::rc::Rc::new(context_wrapper);

        let context = glow::Context::from_loader_function(|s| {
            context_wrapper.get_proc_address(s) as *const _
        });

        let resize_func = {
            let context = context_wrapper.clone();
            move |size:PhysicalSize| context.resize(size)
        };
        let swap_func = {
            let context = context_wrapper.clone();
            move || context.swap_buffers()
        };
        (context, window, el, resize_func, swap_func)
    };

    // Set up dear imgui
    let mut imgui = imgui::Context::create();
    let mut imgui_winit_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    imgui_winit_platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);

    let hidpi_factor = window.hidpi_factor();

    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);

    let imgui_renderer = imgui_opengl::Renderer::new(&mut imgui, &mut gl);

    let mut last_frame = Instant::now();
    let mut demo_open = true;

    let mut size = window.inner_size().to_physical(hidpi_factor);

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                size = new_size.to_physical(hidpi_factor);
                resize_func(size);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            }
            | Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::EventsCleared => {
                let now = Instant::now();
                let delta = now - last_frame;
                let delta_s = delta.as_micros();
                last_frame = now;

                unsafe {
                    gl.viewport(0, 0, size.width as i32, size.height as i32);
                    gl.scissor(0, 0, size.width as i32, size.height as i32);
                    gl.clear_color(0.3f32, 0.3f32, 0.3f32, 0.3f32);
                    gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                }

                imgui_winit_platform
                    .prepare_frame(imgui.io_mut(), &window)
                    .expect("Failed to prepare frame");
                let ui = imgui.frame();

                {
                    let window = imgui::Window::new(im_str!("Hello world"));
                    window
                        .size([300.0, 100.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.text(im_str!("Hello world!"));
                            ui.text(im_str!("This...is...imgui-rs on WGPU!"));
                            ui.separator();
                            let mouse_pos = ui.io().mouse_pos;
                            ui.text(im_str!(
                                "Mouse Position: ({:.1},{:.1})",
                                mouse_pos[0],
                                mouse_pos[1]
                            ));
                        });

                    let window = imgui::Window::new(im_str!("Hello too"));
                    window
                        .size([400.0, 200.0], Condition::FirstUseEver)
                        .position([400.0, 200.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.text(im_str!("Frametime: {}us", delta_s));
                        });

                    ui.show_demo_window(&mut demo_open);
                }

                imgui_renderer.render(&mut gl, ui);
                unsafe {
                    gl.flush();
                }
                swap_func();
            }
            _ => (),
        }

        imgui_winit_platform.handle_event(imgui.io_mut(), &window, &event);
    });
}
