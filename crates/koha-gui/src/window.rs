//! Jendela popup: `winit` untuk jendela dan event, `softbuffer` untuk
//! menampilkan buffer piksel hasil [`render`](crate::render::render).
//!
//! Bagian ini tipis dengan sengaja. Keputusan tentang menu ada di `koha-core`,
//! tata letak dan piksel ada di [`layout`](crate::layout) dan
//! [`render`](crate::render), pemetaan tombol ada di [`input`](crate::input);
//! semuanya bisa dites tanpa jendela. Yang tersisa di sini hanya perekat.

use std::num::NonZeroU32;
use std::rc::Rc;

use koha_core::{Action, Effect, Input, MenuState, State};
use softbuffer::{Context, SoftBufferError, Surface};
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::monitor::MonitorHandle;
use winit::window::{Window, WindowId, WindowLevel};

use crate::input::{FocusGate, KeyAction, map_key};
use crate::layout::{Layout, centered_position};
use crate::render::render;

#[derive(Debug, Error)]
pub enum GuiError {
    #[error("tidak bisa menyiapkan event loop: {0}")]
    EventLoop(#[from] EventLoopError),
    #[error("tidak bisa membuat jendela: {0}")]
    Window(#[from] OsError),
    /// Hanya menyimpan pesannya: `SoftBufferError` berisi pointer mentah dan
    /// bukan `Send + Sync`, sehingga tidak bisa dibungkus `anyhow::Error`.
    #[error("gagal menggambar jendela: {0}")]
    Draw(String),
}

impl From<SoftBufferError> for GuiError {
    fn from(error: SoftBufferError) -> Self {
        Self::Draw(error.to_string())
    }
}

/// Menampilkan menu dan menunggu sampai ditutup.
///
/// - `on_state` dipanggil setiap kali state berubah (tema diganti, item di-ON/OFF)
///   supaya pemanggil bisa menyimpannya.
/// - Mengembalikan `Some(aksi)` bila pengguna memilih aksi, `None` bila menu
///   ditutup tanpa memilih.
///
/// Fungsi ini baru kembali **setelah jendela dihancurkan**. Jadi pemanggil yang
/// menjalankan aksi sesudahnya otomatis melakukannya sesudah jendela tertutup
/// (fokus sudah kembali ke aplikasi sebelumnya), tanpa flag "sedang menutup".
pub fn run(menu: MenuState, on_state: &mut dyn FnMut(&State)) -> Result<Option<Action>, GuiError> {
    let event_loop = EventLoop::new()?;
    // `Wait`: tidur sampai ada event. Tidak ada loop sibuk, jadi CPU idle nol.
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        menu,
        on_state,
        gfx: None,
        gate: FocusGate::default(),
        shift: false,
        outcome: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;

    if let Some(error) = app.error.take() {
        return Err(error);
    }
    let outcome = app.outcome.take();
    // Hancurkan jendela di sini (sebelum `event_loop`, yang baru dilepas di
    // akhir fungsi), bukan menunggu sampai pemanggil selesai.
    drop(app);
    Ok(outcome)
}

/// Segala sesuatu yang hanya ada selama jendela ada.
struct Gfx {
    // Urutan field = urutan drop: surface dulu, baru context dan window.
    surface: Surface<Rc<Window>, Rc<Window>>,
    _context: Context<Rc<Window>>,
    window: Rc<Window>,
    monitor: Option<MonitorHandle>,
    layout: Layout,
}

struct App<'a> {
    menu: MenuState,
    on_state: &'a mut dyn FnMut(&State),
    gfx: Option<Gfx>,
    gate: FocusGate,
    shift: bool,
    outcome: Option<Action>,
    error: Option<GuiError>,
}

impl App<'_> {
    fn create_gfx(&self, event_loop: &ActiveEventLoop) -> Result<Gfx, GuiError> {
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());
        let scale = monitor.as_ref().map_or(1.0, MonitorHandle::scale_factor);
        let layout = Layout::new(self.menu.rows().len(), scale);

        // Dibuat tak terlihat: posisi dan ukuran dibereskan dulu, baru tampil.
        // Ini menghindari jendela muncul sebentar di tempat yang salah (masalah
        // centering di AHK).
        let attributes = Window::default_attributes()
            .with_title(crate::window_title())
            .with_decorations(false)
            .with_resizable(false)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(PhysicalSize::new(layout.width, layout.height))
            .with_visible(false);
        #[cfg(windows)]
        let attributes = {
            use winit::platform::windows::WindowAttributesExtWindows;
            attributes.with_skip_taskbar(true)
        };

        let window = Rc::new(event_loop.create_window(attributes)?);
        let context = Context::new(window.clone())?;
        let surface = Surface::new(&context, window.clone())?;

        let mut gfx = Gfx {
            surface,
            _context: context,
            window,
            monitor,
            layout,
        };
        // Skala jendela sebenarnya bisa berbeda dari skala monitor utama.
        self.fit(&mut gfx);
        gfx.window.set_visible(true);
        gfx.window.focus_window();
        gfx.window.request_redraw();
        Ok(gfx)
    }

    /// Menghitung ulang ukuran dari jumlah baris dan skala, lalu menengahkan.
    fn fit(&self, gfx: &mut Gfx) {
        let layout = Layout::new(self.menu.rows().len(), gfx.window.scale_factor());
        let size = PhysicalSize::new(layout.width, layout.height);
        // Hasil `request_inner_size` diabaikan: ukuran sebenarnya dibaca lagi
        // lewat `inner_size()` saat menggambar.
        let _ = gfx.window.request_inner_size(size);
        if let Some(monitor) = &gfx.monitor {
            let (x, y) = centered_position(
                (monitor.position().x, monitor.position().y),
                (monitor.size().width, monitor.size().height),
                (layout.width, layout.height),
            );
            gfx.window.set_outer_position(PhysicalPosition::new(x, y));
        }
        gfx.layout = layout;
    }

    fn apply(&mut self, input: Input, event_loop: &ActiveEventLoop) {
        match self.menu.update(input) {
            Effect::None => {}
            Effect::Redraw => self.changed(),
            Effect::StateChanged => {
                (self.on_state)(self.menu.state());
                self.changed();
            }
            Effect::Run(action) => {
                self.outcome = Some(action);
                event_loop.exit();
            }
            Effect::Close => event_loop.exit(),
        }
    }

    /// Setelah menu berubah: sesuaikan ukuran bila jumlah baris berubah, lalu gambar.
    fn changed(&mut self) {
        let Some(mut gfx) = self.gfx.take() else {
            return;
        };
        if gfx.layout.rows.len() != self.menu.rows().len() {
            self.fit(&mut gfx);
        }
        gfx.window.request_redraw();
        self.gfx = Some(gfx);
    }

    fn redraw(&mut self) -> Result<(), GuiError> {
        let Some(gfx) = self.gfx.as_mut() else {
            return Ok(());
        };
        let size = gfx.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(()); // jendela diminimalkan / berukuran nol
        };
        gfx.surface.resize(width, height)?;
        let mut buffer = gfx.surface.buffer_mut()?;
        render(
            &mut buffer,
            size.width,
            size.height,
            &gfx.layout,
            &self.menu,
        );
        buffer.present()?;
        Ok(())
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        match self.create_gfx(event_loop) {
            Ok(gfx) => self.gfx = Some(gfx),
            Err(error) => {
                self.error = Some(error);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.apply(Input::Dismiss, event_loop),
            WindowEvent::Focused(focused) => {
                if let Some(input) = self.gate.on_focus_changed(focused) {
                    self.apply(input, event_loop);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift = modifiers.state().shift_key();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        repeat,
                        ..
                    },
                ..
            } => {
                if let KeyAction::Send(input) = map_key(&logical_key, self.shift, repeat) {
                    self.apply(input, event_loop);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(mut gfx) = self.gfx.take() {
                    self.fit(&mut gfx);
                    gfx.window.request_redraw();
                    self.gfx = Some(gfx);
                }
            }
            WindowEvent::Resized(_) => {
                if let Some(gfx) = &self.gfx {
                    gfx.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.error = Some(error);
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
