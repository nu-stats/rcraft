//! A real window drawn on the CPU, for computers whose graphics driver does
//! not work (a failed or missing driver, a remote session, a virtual
//! machine). It is the same `App`; only the drawing differs: egui's output is
//! filled into a pixel buffer by `softraster` and handed to the system with
//! `softbuffer`.
//!
//! Slower than the graphics-card window, and fine for RCraft's mostly static
//! panes. Release builds are much faster than debug builds.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::app::App;
use crate::softraster::{Canvas, Textures};

type Surface = softbuffer::Surface<Arc<Window>, Arc<Window>>;

struct Live {
    window: Arc<Window>,
    surface: Surface,
    state: egui_winit::State,
}

struct SoftApp {
    ctx: egui::Context,
    app: App,
    textures: Textures,
    live: Option<Live>,
    /// When egui next wants a frame, if it asked for one in the future.
    next_repaint: Option<Instant>,
    failure: Option<String>,
}

/// Opens the software-drawn window and runs until it is closed.
pub fn run() -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let ctx = egui::Context::default();
    let app = App::new(&ctx);
    let mut soft = SoftApp {
        ctx,
        app,
        textures: Textures::default(),
        live: None,
        next_repaint: None,
        failure: None,
    };
    event_loop.run_app(&mut soft).map_err(|e| e.to_string())?;
    match soft.failure {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

impl ApplicationHandler for SoftApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        match self.open(event_loop) {
            Ok(live) => {
                live.window.request_redraw();
                self.live = Some(live);
            }
            Err(why) => {
                self.failure = Some(why);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let response = live.state.on_window_event(&live.window, &event);
        if response.repaint {
            live.window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => live.window.request_redraw(),
            WindowEvent::RedrawRequested => {
                if let Err(why) = self.redraw() {
                    self.failure = Some(why);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(live) = self.live.as_ref() else {
            return;
        };
        match self.next_repaint {
            Some(at) if at <= Instant::now() => {
                self.next_repaint = None;
                live.window.request_redraw();
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

impl SoftApp {
    fn open(&self, event_loop: &ActiveEventLoop) -> Result<Live, String> {
        let attributes = Window::default_attributes()
            .with_title("RCraft")
            .with_inner_size(LogicalSize::new(1240.0, 780.0))
            .with_min_inner_size(LogicalSize::new(760.0, 480.0));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| format!("could not open a window: {e}"))?,
        );
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| format!("could not start software drawing: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| format!("could not start software drawing: {e}"))?;
        let state = egui_winit::State::new(
            self.ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        Ok(Live {
            window,
            surface,
            state,
        })
    }

    /// Runs one egui frame and puts the result on the screen.
    fn redraw(&mut self) -> Result<(), String> {
        let Some(live) = self.live.as_mut() else {
            return Ok(());
        };
        let raw = live.state.take_egui_input(&live.window);
        let output = self.ctx.run(raw, |ctx| self.app.ui(ctx));
        live.state
            .handle_platform_output(&live.window, output.platform_output);
        self.textures.apply(&output.textures_delta);

        let delay = output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
        self.next_repaint = (delay < Duration::from_secs(3600)).then(|| Instant::now() + delay);

        let size = live.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Ok(()); // minimised
        };
        let ppp = output.pixels_per_point;
        let mut canvas = Canvas::new(w.get() as usize, h.get() as usize, Color32::BLACK);
        let primitives = self.ctx.tessellate(output.shapes, ppp);
        canvas.draw(&primitives, &self.textures, ppp);

        live.surface
            .resize(w, h)
            .map_err(|e| format!("could not resize the drawing surface: {e}"))?;
        let mut buffer = live
            .surface
            .buffer_mut()
            .map_err(|e| format!("could not get the drawing buffer: {e}"))?;
        for (dst, src) in buffer.iter_mut().zip(canvas.pixels.iter()) {
            let [r, g, b, _] = src.to_array();
            *dst = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
        }
        buffer
            .present()
            .map_err(|e| format!("could not show the window contents: {e}"))
    }
}
