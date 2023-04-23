use std::sync::mpsc::Receiver;
use winit::platform::windows::EventLoopBuilderExtWindows;

struct Gui {
    rx: Receiver<Message>,
}

impl Gui {
    pub fn new(rx: Receiver<Message>) -> Self {
        Self { rx }
    }
}
pub enum Message {
    Start,
}

impl eframe::App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
        });
    }
}

fn do_gui(rx: Receiver<Message>) {
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

pub fn run(rx: Receiver<Message>) {
    let gui_thread_entry = {
        //        let msg = rx.recv().unwrap();
        do_gui(rx);
    };
    std::thread::spawn(move || gui_thread_entry);
}
