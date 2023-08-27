use crate::dcs;
use crate::gui;
use mlua::Lua;
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct App {
    thread: Option<JoinHandle<()>>,
    gui: gui::Handle,
    ownship_type: dcs::AircraftId,
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
            ownship_type: dcs::AircraftId::Unknown(String::from("")),
        };
        me
    }

    pub fn stop(&mut self) {
        let thread_finish = std::mem::take(&mut self.thread);
        self.gui.stop();
        thread_finish.unwrap().join().unwrap();
        log::info!("Main thread stopped");
    }

    pub fn on_frame(&mut self, lua: &Lua) {
        let ownship_type = dcs::get_ownship_type(lua);
        if self.ownship_type != ownship_type {
            log::info!("Got new aircraft type {:?}", ownship_type);
            self.ownship_type = ownship_type.clone();
            if self.gui.is_running() {
                let result = self.gui.set_ownship_type(ownship_type);
                if result.is_err() {
                    self.gui.stop();
                }
            }
        }
    }
}

fn app_thread_entry() {}
