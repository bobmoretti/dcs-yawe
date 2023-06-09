use crate::dcs;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use winit::platform::windows::EventLoopBuilderExtWindows;

// Publicly-facing handle to GUI thread
#[derive(Debug)]
pub struct Handle {
    tx: Sender<Message>,
    thread: Option<std::thread::JoinHandle<()>>,
    context: egui::Context,
}

struct Gui {
    rx: Receiver<Message>,
    aircraft_name: &'static str,
}

impl Gui {
    pub fn new(rx: Receiver<Message>) -> Self {
        Self {
            rx: rx,
            aircraft_name: "",
        }
    }
}

fn aircraft_display_name(kind: dcs::AircraftType) -> &'static str {
    match kind {
        dcs::AircraftType::F_16C_50 => "F-16C block 50",
        dcs::AircraftType::UNKNOWN => "",
    }
}

pub enum Message {
    Stop,
    UpdateOwnship(dcs::AircraftType),
}

impl Handle {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Message>();
        let tx_clone = tx.clone();
        let context = egui::Context::default();
        let context_clone = context.clone();
        let thread = std::thread::spawn(move || {
            do_gui(rx, context);
        });
        Handle {
            tx: tx_clone,
            thread: Some(thread),
            context: context_clone,
        }
    }

    pub fn set_ownship_type(&self, kind: dcs::AircraftType) -> Result<(), SendError<Message>> {
        self.tx.send(Message::UpdateOwnship(kind))?;
        self.context.request_repaint();
        Ok(())
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
        self.thread.is_some()
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let msg = self.rx.try_recv();

        if let Ok(m) = msg {
            match m {
                Message::Stop => {
                    log::info!("Gui: received a `Stop` message");
                    frame.close();
                    return;
                }
                Message::UpdateOwnship(kind) => self.aircraft_name = aircraft_display_name(kind),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DCS YAWE");
            ui.horizontal(|ui| {
                ui.label("Aircraft type:");
                ui.label(self.aircraft_name);
            });
        });
    }
}

fn do_gui(rx: Receiver<Message>, context: egui::Context) {
    log::info!("Starting gui");
    let mut native_options = eframe::NativeOptions::default();
    native_options.event_loop_builder = Some(Box::new(|builder| {
        log::debug!("Calling eframe event loop hook");
        builder.with_any_thread(true);
    }));
    native_options.renderer = eframe::Renderer::Wgpu;
    native_options.context = Some(context);
    log::info!("Spawning GUI thread");

    let gui = Gui::new(rx);

    eframe::run_native(
        "DCS Yawe",
        native_options,
        Box::new(move |_cc| Box::new(gui)),
    )
    .expect("Eframe ran successfully");

    log::info!("Gui closed");
}
