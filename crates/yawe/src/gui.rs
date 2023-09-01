use crate::app;
use crate::dcs;
use egui_backend::{egui, BackendConfig, GfxBackend, UserApp, WindowBackend};
use egui_render_glow::GlowBackend;
use egui_window_glfw_passthrough::GlfwBackend;
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{self, Receiver, SendError, Sender};

// Publicly-facing handle to GUI thread
#[derive(Debug)]
pub struct Handle {
    tx: Sender<Message>,
    thread: Option<std::thread::JoinHandle<()>>,
    context: egui::Context,
}

struct Gui {
    rx: Receiver<Message>,
    tx: Sender<app::AppMessage>,
    aircraft_name: &'static str,
    canopy_state: f32,
    pub egui_context: egui::Context,
    pub glow_backend: GlowBackend,
    pub glfw_backend: GlfwBackend,
}

impl Gui {
    pub fn new(rx: Receiver<Message>, tx: Sender<app::AppMessage>, context: egui::Context) -> Self {
        let mut glfw_backend = GlfwBackend::new(Default::default(), BackendConfig::default());
        // creating gfx backend. It uses Window backend to load things like fn pointers
        // or window handle for swapchain etc.. behind the scenes.
        let glow_backend = GlowBackend::new(&mut glfw_backend, Default::default());
        Self {
            rx: rx,
            tx: tx,
            aircraft_name: "",
            canopy_state: 1.0,
            glfw_backend: glfw_backend,
            glow_backend: glow_backend,
            egui_context: context,
        }
    }

    fn close(&mut self) {
        self.glfw_backend.window.set_should_close(true);
    }

    fn make_debug_widget(&mut self, ui: &mut egui::Ui) {
        ui.label("Debug switches:");
        egui::Grid::new("debug_grid").show(ui, |ui| {
            for switch_info in &dcs::mig21bis::SWITCH_INFO_MAP {
                let s: &'static str = switch_info._switch.into();
                ui.label(s);
                if ui.button("Set").clicked() {
                    // set
                }
                if ui.button("Get").clicked() {
                    // get
                }
                ui.end_row();
            }
        });
    }
}

fn aircraft_display_name(kind: dcs::AircraftId) -> &'static str {
    match kind {
        dcs::AircraftId::F_16C_50 => "F-16C block 50",
        dcs::AircraftId::A_10C => "A-10C",
        dcs::AircraftId::A_10C_2 => "A-10C II",
        dcs::AircraftId::AH_64D_BLK_II => "AH-64D Apache",
        dcs::AircraftId::AJS37 => "AJS37 Viggen",
        dcs::AircraftId::AV8BNA => "AV8BNA Harrier",
        dcs::AircraftId::F_14B => "F-14B Tomcat",
        dcs::AircraftId::F_15ESE => "F-15E Strike Eagle",
        dcs::AircraftId::F_15ESE_WSO => "F-15E Strike Eagle (WSO)",
        dcs::AircraftId::FA_18C_hornet => "F/A-18C Hornet",
        dcs::AircraftId::M_2000C => "Mirage 2000C",
        dcs::AircraftId::Mi_24P => "Mi-24P \"Hind E\"",
        dcs::AircraftId::Mi_8MT => "Mi-8MT \"Hip\"",
        dcs::AircraftId::Mi_8MT_Copilot => "Mi-8MT \"Hip\" (Copilot)",
        dcs::AircraftId::Mi_8MT_FO => "Mi-8MT \"Hip\" (First Officer)",
        dcs::AircraftId::MiG_21Bis => "MiG-21Bis",
        dcs::AircraftId::SA342L => "SA342L Gazelle",
        dcs::AircraftId::Su_25 => "Su-25 \"Frogfoot\"",
        dcs::AircraftId::Su_25T => "Su-25T \"Frogfoot\"",
        dcs::AircraftId::UH_1H => "UH-1H Huey",
        // TODO: this is a hack
        dcs::AircraftId::Unknown(s) => s.leak(),
    }
}

pub enum Message {
    Stop,
    UpdateCanopyState(f32),
    UpdateOwnship(dcs::AircraftId),
}

impl Handle {
    pub fn new(tx_to_app: Sender<app::AppMessage>) -> Self {
        let (tx, rx) = mpsc::channel::<Message>();
        let tx_clone = tx.clone();
        let context = egui::Context::default();
        let context_clone = context.clone();
        let thread = std::thread::spawn(move || {
            do_gui(rx, tx_to_app, context);
        });
        Handle {
            tx: tx_clone,
            thread: Some(thread),
            context: context_clone,
        }
    }

    pub fn set_ownship_type(&self, kind: dcs::AircraftId) -> Result<(), SendError<Message>> {
        self.tx.send(Message::UpdateOwnship(kind))?;
        self.context.request_repaint();
        Ok(())
    }

    pub fn set_canopy_state(&self, state: f32) {
        if let Err(e) = self.tx.send(Message::UpdateCanopyState(state)) {
            log::warn!("Sending canopy state failed with {:?}", e);
        };
        self.context.request_repaint();
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
        self.glfw_backend.window.set_floating(true);

        // process all pending messages in the queue each frame of the GUI
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Message::Stop => {
                    log::info!("Gui: received a `Stop` message");
                    self.close();
                    return;
                }
                Message::UpdateOwnship(kind) => self.aircraft_name = aircraft_display_name(kind),
                Message::UpdateCanopyState(state) => {
                    if self.canopy_state != state {
                        log::info!("NEW STATE: {state}");
                    }
                    self.canopy_state = state
                }
            }
        }

        egui::CentralPanel::default().show(&ctx, |ui| {
            ui.heading("DCS YAWE");
            ui.horizontal(|ui| {
                ui.label("Aircraft type:");
                ui.label(self.aircraft_name);
            });
            if (ui.button("Start")).clicked() {
                let _ = self.tx.send(app::AppMessage::StartupAircraft);
            }
            ui.horizontal(|ui| {
                ui.label("Canopy State");
                ui.add(egui::ProgressBar::new(self.canopy_state));
            });
            self.make_debug_widget(ui);
        });
    }
}

fn do_gui(rx: Receiver<Message>, tx: Sender<app::AppMessage>, context: egui::Context) {
    log::info!("Starting gui");
    let gui = Gui::new(rx, tx, context);
    <Gui as UserApp>::UserWindowBackend::run_event_loop(gui);

    log::info!("Gui closed");
}
