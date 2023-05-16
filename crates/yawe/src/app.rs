use crate::gui;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct App {
    thread: Option<JoinHandle<()>>,
    gui: gui::Handle,
}

impl App {
    // Start the main application (scoped) thread, return an interface handle to
    // allow the outside world to talk to it.
    pub fn new() -> Self {
        // create a new gui handle
        let gui = gui::Handle::new();
        let me = Self {
            thread: Some(std::thread::spawn(|| app_thread_entry())),
            gui: gui,
        };
        me
    }

    pub fn stop(&mut self) {
        let thread_finish = std::mem::take(&mut self.thread);
        self.gui.stop();
        thread_finish.unwrap().join().unwrap();
        log::info!("Main thread stopped");
    }
}

fn app_thread_entry() {}
