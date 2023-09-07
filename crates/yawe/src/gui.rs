use crate::app;
use crate::dcs;
use egui_backend::{egui, BackendConfig, GfxBackend, UserApp, WindowBackend};
use egui_render_glow::GlowBackend;
use egui_window_glfw_passthrough::glfw;
use egui_window_glfw_passthrough::glfw::Context;
use egui_window_glfw_passthrough::GlfwBackend;
use mlua::Lua;
use offload::TaskSender;
use std::sync::mpsc::{self, Receiver, Sender};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, POINTS, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::WindowsAndMessaging::HTRIGHT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowLongPtrW, GetWindowRect, SetWindowLongPtrW, SetWindowPos, GWLP_WNDPROC,
    GWL_STYLE, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTLEFT, HTTOP, HTTOPLEFT,
    HTTOPRIGHT, NCCALCSIZE_PARAMS, SWP_FRAMECHANGED, SWP_NOMOVE, WM_NCACTIVATE, WM_NCCALCSIZE,
    WM_NCHITTEST, WM_NCPAINT, WNDPROC, WS_CAPTION, WS_THICKFRAME,
};
struct Gui {
    rx: Receiver<Message>,
    tx: Sender<app::AppMessage>,
    to_dcs_gamegui: TaskSender<Lua>,
    _to_dcs_export: TaskSender<Lua>,
    aircraft_name: String,
    startup_progress: f32,
    pub egui_context: egui::Context,
    pub glow_backend: GlowBackend,
    pub glfw_backend: GlfwBackend,
    switch_vals: Vec<String>,
    is_on_top: bool,
    debug_widget_visible: bool,
    startup_text: String,
    paused: bool,
}

// The following hackery is based on the thread:
// https://reddit.com/r/opengl/comments/13x3sw0/custom_title_bar_with_glfw/

// this needs to be global (I don't think there is any other way to get it into
// an extern "C" function)
static mut GLFW_PROC: WNDPROC = None;

fn hit_test(
    mouse_pos: &POINT,
    window_rect: &RECT,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let border_width = 8;
    let caption_height = 32;
    if mouse_pos.y >= window_rect.bottom - border_width {
        if mouse_pos.x <= border_width {
            return LRESULT(HTBOTTOMLEFT as isize);
        } else if mouse_pos.x >= window_rect.right - border_width {
            return LRESULT(HTBOTTOMRIGHT as isize);
        } else {
            return LRESULT(HTBOTTOM as isize);
        }
    } else if mouse_pos.y <= border_width {
        if mouse_pos.x <= border_width {
            return LRESULT(HTTOPLEFT as isize);
        } else if mouse_pos.x >= window_rect.right - border_width {
            return LRESULT(HTTOPRIGHT as isize);
        } else {
            return LRESULT(HTTOP as isize);
        }
    } else if mouse_pos.y <= border_width + caption_height {
        return LRESULT(HTCAPTION as isize);
    } else if mouse_pos.x <= border_width {
        return LRESULT(HTLEFT as isize);
    } else if mouse_pos.x >= window_rect.right - border_width {
        return LRESULT(HTRIGHT as isize);
    }
    unsafe { GLFW_PROC.unwrap()(hwnd, msg, wparam, lparam) }
}

unsafe extern "system" fn modified_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCALCSIZE => {
            if wparam.0 == 1 && lparam.0 != 0 {
                let params = lparam.0 as *mut NCCALCSIZE_PARAMS;
                (*params).rgrc[0].top += 1;
                (*params).rgrc[0].right -= 1;
                (*params).rgrc[0].bottom -= 1;
                (*params).rgrc[0].left += 1;
            }
            LRESULT(0)
        }
        WM_NCHITTEST => {
            let x: i16 = (lparam.0 & 0xFFFF) as i16;
            let y: i16 = ((lparam.0 >> 16) & 0xFFFF) as i16;
            let mouse_pos = POINTS { x, y };
            let mut client_mouse_pos = POINT {
                x: mouse_pos.x as i32,
                y: mouse_pos.y as i32,
            };
            ScreenToClient(hwnd, &mut client_mouse_pos as *mut POINT);
            let mut window_rect = RECT::default();
            GetClientRect(hwnd, &mut window_rect as *mut RECT);
            let result = hit_test(&client_mouse_pos, &window_rect, hwnd, msg, wparam, lparam);
            result
        }
        WM_NCPAINT => LRESULT(0),
        WM_NCACTIVATE => LRESULT(0),
        _ => GLFW_PROC.unwrap()(hwnd, msg, wparam, lparam),
    }
}

fn disable_titlebar(window: &glfw::Window) {
    let window_handle = unsafe { HWND(glfw::ffi::glfwGetWin32Window(window.window_ptr()) as _) };
    let mut style = unsafe { GetWindowLongPtrW(window_handle, GWL_STYLE) };
    style |= WS_THICKFRAME.0 as isize;
    style &= !WS_CAPTION.0 as isize;
    unsafe { SetWindowLongPtrW(window_handle, GWL_STYLE, style) };
    let mut rect: RECT = RECT::default();
    unsafe { GetWindowRect(window_handle, &mut rect) };
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    let raw = unsafe { GetWindowLongPtrW(window_handle, GWLP_WNDPROC) };
    unsafe {
        GLFW_PROC = Some(std::mem::transmute(raw));
    };
    unsafe { SetWindowLongPtrW(window_handle, GWLP_WNDPROC, modified_proc as isize) };
    unsafe {
        SetWindowPos(
            window_handle,
            HWND(0 as isize),
            0,
            0,
            width,
            height,
            SWP_FRAMECHANGED | SWP_NOMOVE,
        )
    };
}

impl Gui {
    pub fn new(
        rx: Receiver<Message>,
        tx: Sender<app::AppMessage>,
        to_dcs_gamegui: TaskSender<Lua>,
        to_dcs_export: TaskSender<Lua>,
        context: egui::Context,
    ) -> Self {
        let mut glfw_backend = GlfwBackend::new(Default::default(), BackendConfig::default());
        glfw_backend.window.set_decorated(false);
        glfw_backend.window.set_title("DCS YAWE");
        disable_titlebar(&glfw_backend.window);

        // creating gfx backend. It uses Window backend to load things like fn pointers
        // or window handle for swapchain etc.. behind the scenes.
        let glow_backend = GlowBackend::new(&mut glfw_backend, Default::default());
        Self {
            rx: rx,
            tx: tx,
            to_dcs_gamegui,
            _to_dcs_export: to_dcs_export,
            aircraft_name: String::from(""),
            startup_progress: 0.0,
            glfw_backend: glfw_backend,
            glow_backend: glow_backend,
            egui_context: context,
            switch_vals: {
                let mut v = Vec::with_capacity(dcs::mig21bis::Switch::NumSwitches as usize);
                v.resize(dcs::mig21bis::Switch::NumSwitches as usize, String::new());
                v
            },
            is_on_top: true,
            debug_widget_visible: false,
            startup_text: String::default(),
            paused: false,
        }
    }

    fn close(&mut self) {
        self.glfw_backend.window.set_should_close(true);
    }

    fn make_debug_widget(&mut self, ui: &mut egui::Ui) {
        ui.label("Debug switches:");
        egui::Grid::new("debug_grid").show(ui, |ui| {
            for (ii, &ref switch_info) in dcs::mig21bis::SWITCH_INFO_MAP.iter().enumerate() {
                let s: &'static str = switch_info.switch.into();
                ui.label(s);
                let val = &mut (self.switch_vals[ii]);
                if ui.button("Set").clicked() {
                    let result = val.parse::<f32>();
                    if let Ok(state) = result {
                        let _ = self.to_dcs_gamegui.send(move |lua| {
                            dcs::mig21bis::set_switch_state(lua, switch_info.switch, state)
                        });
                    }
                }
                if ui.button("Get").clicked() {
                    let result = self
                        .to_dcs_gamegui
                        .send(|lua| dcs::mig21bis::get_switch_state(lua, switch_info.switch))
                        .wait();
                    if let Ok(state) = result {
                        val.replace_range(.., state.unwrap_or_default().to_string().as_str());
                    }
                }
                ui.add(egui::TextEdit::singleline(val));
                ui.end_row();
            }
        });
    }
}

impl UserApp for Gui {
    type UserGfxBackend = GlowBackend;
    type UserWindowBackend = GlfwBackend;
    fn get_all(
        &mut self,
    ) -> (
        &mut Self::UserWindowBackend,
        &mut Self::UserGfxBackend,
        &egui::Context,
    ) {
        (
            &mut self.glfw_backend,
            &mut self.glow_backend,
            &self.egui_context,
        )
    }

    fn gui_run(&mut self) {
        let ctx = self.egui_context.clone();

        // process all pending messages in the queue each frame of the GUI
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Message::Stop => {
                    log::info!("Gui: received a `Stop` message");
                    self.close();
                    return;
                }
                Message::UpdateOwnship(kind) => {
                    let num_switches = match kind {
                        dcs::AircraftId::MiG_21Bis => dcs::mig21bis::Switch::NumSwitches as usize,
                        _ => 0 as usize,
                    };
                    let mut v = Vec::with_capacity(num_switches as usize);
                    v.resize(num_switches as usize, String::new());
                    self.switch_vals = v;
                    self.aircraft_name = aircraft_display_name(kind);
                }
                Message::UpdateStartupProgress(progress) => self.startup_progress = progress,
                Message::UpdateStartupText(s) => self.startup_text = s,
                Message::Paused => self.paused = true,
                Message::Unpaused => self.paused = false,
            }
        }
        self.glfw_backend.window.set_floating(self.is_on_top);
        self.glfw_backend.window.set_decorated(false);

        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.heading("DCS YAWE");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Aircraft type:");
                ui.label(self.aircraft_name.as_str());
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Autostart");
                let enabled = !self.paused;
                let start_button = ui.add_enabled(enabled, egui::Button::new("Start"));
                if start_button.clicked() {
                    let _ = self
                        .tx
                        .send(app::AppMessage::FsmEvent(app::FsmMessage::StartupAircraft));
                }

                ui.add(
                    egui::ProgressBar::new(self.startup_progress)
                        .text(self.startup_text.as_str())
                        .animate(self.startup_progress > 0.0),
                );
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.debug_widget_visible, "Debug panel");
                ui.checkbox(&mut self.is_on_top, "Always on top");
            });

            if self.debug_widget_visible {
                ui.separator();
                self.make_debug_widget(ui);
            }
        });
    }
}

fn do_gui(
    rx: Receiver<Message>,
    tx: Sender<app::AppMessage>,
    to_dcs_gamegui: TaskSender<Lua>,
    to_dcs_export: TaskSender<Lua>,
    context: egui::Context,
) {
    log::info!("Starting gui");
    let gui = Gui::new(rx, tx, to_dcs_gamegui, to_dcs_export, context);
    <Gui as UserApp>::UserWindowBackend::run_event_loop(gui);

    log::info!("Gui closed");
}

fn aircraft_display_name(kind: dcs::AircraftId) -> String {
    match kind {
        dcs::AircraftId::F_16C_50 => String::from("F-16C block 50"),
        dcs::AircraftId::A_10C => String::from("A-10C"),
        dcs::AircraftId::A_10C_2 => String::from("A-10C II"),
        dcs::AircraftId::AH_64D_BLK_II => String::from("AH-64D Apache"),
        dcs::AircraftId::AJS37 => String::from("AJS37 Viggen"),
        dcs::AircraftId::AV8BNA => String::from("AV8BNA Harrier"),
        dcs::AircraftId::F_14B => String::from("F-14B Tomcat"),
        dcs::AircraftId::F_15ESE => String::from("F-15E Strike Eagle"),
        dcs::AircraftId::F_15ESE_WSO => String::from("F-15E Strike Eagle (WSO)"),
        dcs::AircraftId::FA_18C_hornet => String::from("F/A-18C Hornet"),
        dcs::AircraftId::M_2000C => String::from("Mirage 2000C"),
        dcs::AircraftId::Mi_24P => String::from("Mi-24P \"Hind E\""),
        dcs::AircraftId::Mi_8MT => String::from("Mi-8MT \"Hip\""),
        dcs::AircraftId::Mi_8MT_Copilot => String::from("Mi-8MT \"Hip\" (Copilot)"),
        dcs::AircraftId::Mi_8MT_FO => String::from("Mi-8MT \"Hip\" (First Officer)"),
        dcs::AircraftId::MiG_21Bis => String::from("MiG-21Bis"),
        dcs::AircraftId::SA342L => String::from("SA342L Gazelle"),
        dcs::AircraftId::Su_25 => String::from("Su-25 \"Frogfoot\""),
        dcs::AircraftId::Su_25T => String::from("Su-25T \"Frogfoot\""),
        dcs::AircraftId::UH_1H => String::from("UH-1H Huey"),
        // TODO: this is a hack
        dcs::AircraftId::Unknown(s) => s,
    }
}

enum Message {
    Stop,
    UpdateStartupProgress(f32),
    UpdateOwnship(dcs::AircraftId),
    UpdateStartupText(String),
    Paused,
    Unpaused,
}

// Need a separate struct to abstract the subset of functionality that cannot be
// sent across threads, since `Handle` contains a handle to the GUI thread.
#[derive(Clone, Debug)]
pub struct TxHandle {
    context: egui::Context,
    tx: Sender<Message>,
}

impl TxHandle {
    pub fn set_ownship_type(&self, kind: dcs::AircraftId) {
        let _ = self.tx.send(Message::UpdateOwnship(kind));
        self.context.request_repaint();
    }

    pub fn set_startup_progress(&self, progress: f32) {
        let _ = self.tx.send(Message::UpdateStartupProgress(progress));
        self.context.request_repaint();
    }

    pub fn set_startup_text(&self, text: &'static str) {
        let _ = self.tx.send(Message::UpdateStartupText(String::from(text)));
        self.context.request_repaint();
    }

    pub fn set_paused(&self) {
        let _ = self.tx.send(Message::Paused);
        self.context.request_repaint();
    }

    pub fn set_unpaused(&self) {
        let _ = self.tx.send(Message::Unpaused);
        self.context.request_repaint();
    }
}

// Publicly-facing handle to GUI thread
#[derive(Debug)]
pub struct Handle {
    tx: Sender<Message>,
    thread: Option<std::thread::JoinHandle<()>>,
    context: egui::Context,
}

impl Handle {
    pub fn new(
        tx_to_app: Sender<app::AppMessage>,
        to_dcs_gamegui: TaskSender<Lua>,
        to_dcs_export: TaskSender<Lua>,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Message>();
        let tx_clone = tx.clone();
        let context = egui::Context::default();
        let context_clone = context.clone();
        let thread = std::thread::Builder::new()
            .name("yawe-gui".to_string())
            .spawn(move || {
                do_gui(rx, tx_to_app, to_dcs_gamegui, to_dcs_export, context);
            })
            .unwrap();
        Handle {
            tx: tx_clone,
            thread: Some(thread),
            context: context_clone,
        }
    }

    pub fn tx_handle(&self) -> TxHandle {
        TxHandle {
            context: self.context.clone(),
            tx: self.tx.clone(),
        }
    }

    pub fn set_ownship_type(&self, kind: dcs::AircraftId) {
        let _ = self.tx_handle().set_ownship_type(kind);
    }

    pub fn _set_startup_progress(&self, progress: f32) {
        self.tx_handle().set_startup_progress(progress)
    }

    pub fn _set_startup_text(&self, text: &'static str) {
        self.tx_handle().set_startup_text(text)
    }

    pub fn set_paused(&self) {
        self.tx_handle().set_paused()
    }

    pub fn set_unpaused(&self) {
        self.tx_handle().set_unpaused()
    }

    pub fn stop(&mut self) {
        log::info!("GUI stop called!");
        let tx = &self.tx;
        tx.send(Message::Stop).unwrap_or(());
        self.context.request_repaint();

        if self.thread.is_some() {
            let thread = self.thread.take().unwrap();
            thread.join().unwrap();
        }
    }

    pub fn is_running(&self) -> bool {
        match &self.thread {
            Some(t) => !t.is_finished(),
            None => false,
        }
    }
}
