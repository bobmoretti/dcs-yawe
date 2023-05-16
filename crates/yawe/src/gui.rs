use std::sync::mpsc::{self, Receiver, Sender};
use winit::platform::windows::EventLoopBuilderExtWindows;

// Publicly-facing handle to GUI thread
#[derive(Debug)]
pub struct Handle {
    tx: Sender<Message>,
    thread: Option<std::thread::JoinHandle<()>>,
}

struct Gui {
    rx: Receiver<Message>,
}

impl Gui {
    pub fn new(rx: Receiver<Message>) -> Self {
        Self { rx }
    }
}

enum Message {
    Stop,
}

impl Handle {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Message>();
        let tx_clone = tx.clone();
        let thread = std::thread::spawn(move || {
            do_gui(rx);
        });
        Handle {
            tx: tx_clone,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) {
        let tx = &self.tx;
        tx.send(Message::Stop).unwrap_or(());
        let thread = self.thread.take().unwrap();
        thread.join().unwrap()
    }
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let msg = self.rx.try_recv();

        if let Ok(Message::Stop) = msg {
            log::info!("Gui: received a `Stop` message");
            frame.close();
            return;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
        });
    }
}

fn do_gui(rx: Receiver<Message>) {
    log::info!("Starting gui");
    let mut native_options = eframe::NativeOptions::default();
    native_options.event_loop_builder = Some(Box::new(|builder| {
        log::debug!("Calling eframe event loop hook");
        builder.with_any_thread(true);
    }));
    native_options.renderer = eframe::Renderer::Wgpu;
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
