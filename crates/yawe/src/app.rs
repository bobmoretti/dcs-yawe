use crate::dcs;
use crate::gui;
use mlua::Lua;
use offload::{PackagedTask, TaskSender};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct App {
    thread: Option<JoinHandle<()>>,
    gui: gui::Handle,
    ownship_type: dcs::AircraftId,
    dcs_worker_rx: Receiver<PackagedTask<Lua>>,
}

pub enum AppMessage {
    StartAircraft,
}

pub enum AppReply {
    StartFinished,
}

impl App {
    // Start the main application (scoped) thread, return an interface handle to
    // allow the outside world to talk to it.
    pub fn new() -> Self {
        // create a new gui handle
        let (dcs_worker_tx, dcs_worker_rx) = TaskSender::new();
        let (tx_to_app, rx_from_gui) = channel::<AppMessage>();
        let (tx_to_gui, rx_from_app) = channel::<AppReply>();

        let gui = gui::Handle::new(tx_to_app);

        let me = Self {
            thread: Some(std::thread::spawn(|| {
                app_thread_entry(dcs_worker_tx, rx_from_gui, tx_to_gui)
            })),
            gui: gui,
            ownship_type: dcs::AircraftId::Unknown(String::from("")),
            dcs_worker_rx,
        };
        me
    }

    pub fn stop(&mut self) {
        let thread_finish = std::mem::take(&mut self.thread);
        self.gui.stop();
        thread_finish.unwrap().join().unwrap();
        log::info!("Main thread stopped");
    }

    pub fn on_frame(&mut self, lua: &Lua) -> i32 {
        let ownship_type = match dcs::get_ownship_type(lua) {
            Ok(t) => t,
            Err(_) => dcs::AircraftId::Unknown(String::from("")),
        };

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

        if let Err(e) = self.dcs_worker_rx.try_recv().map(|job| job(lua)) {
            if let TryRecvError::Empty = e {
            } else {
                log::warn!("Offload tick failed with error {:?}", e);
            }
        }

        if self.gui.is_running() {
            0
        } else {
            -1
        }
    }
}
fn app_thread_entry(
    sender_to_dcs: TaskSender<Lua>,
    rx_from_gui: Receiver<AppMessage>,
    tx_to_gui: Sender<AppReply>,
) -> ! {
    loop {
        let mesg = rx_from_gui.recv();
        if mesg.is_err() {
            continue;
        }
        match mesg.as_ref().unwrap() {
            AppMessage::StartAircraft => sender_to_dcs.send(|lua| start_jet(&lua)).wait().unwrap(),
        };
    }
}

fn start_jet(lua: &Lua) {
    let value = dcs::get_switch_state(lua, 0, 50);
    if let Ok(v) = value {
        log::info!("RPM state: {v}");
    } else {
        log::warn!("Failed to read state of device with {:?}", value);
    }
}
