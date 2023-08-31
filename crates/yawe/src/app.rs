use crate::dcs;
use crate::gui;
use mlua::Lua;
use offload::{PackagedTask, TaskSender};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;

#[derive(Debug)]
pub struct App {
    thread: Option<JoinHandle<()>>,
    tx_to_app: Sender<AppMessage>,
    gui: gui::Handle,
    ownship_type: dcs::AircraftId,
    dcs_worker_rx: Receiver<PackagedTask<Lua>>,
}

pub enum AppMessage {
    StartupAircraft,
    InterruptAircraftStart,
    StopApp,
}

pub enum AppReply {
    StartupComplete,
}

impl App {
    // Start the main application (scoped) thread, return an interface handle to
    // allow the outside world to talk to it.
    pub fn new() -> Self {
        // create a new gui handle
        let (dcs_worker_tx, dcs_worker_rx) = TaskSender::new();
        let (tx_to_app, rx_from_gui) = channel::<AppMessage>();
        let (tx_to_gui, rx_from_app) = channel::<AppReply>();

        let gui = gui::Handle::new(tx_to_app.clone());

        let me = Self {
            thread: Some(std::thread::spawn(|| {
                app_thread_entry(dcs_worker_tx, rx_from_gui, tx_to_gui)
            })),
            tx_to_app: tx_to_app.clone(),
            gui: gui,
            ownship_type: dcs::AircraftId::Unknown(String::from("")),
            dcs_worker_rx,
        };
        me
    }

    pub fn stop(&mut self) {
        if let Err(e) = self.tx_to_app.send(AppMessage::StopApp) {
            log::warn!(
                "Warning, could not send Stop message to app thread, error was {:?}",
                e
            );
        };
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

        let _ = self.dcs_worker_rx.try_recv().map(|job| job(lua));

        if self.gui.is_running() {
            0
        } else {
            -1
        }
    }

    pub fn on_frame_export(&mut self, lua: &Lua) -> i32 {
        match dcs::get_cockpit_param(lua, "BASE_SENSOR_CANOPY_POS") {
            Ok(canopy_state) => self.gui.set_canopy_state(canopy_state),
            Err(e) => log::warn!("Error {:?} getting canopy state", e),
        }
        0
    }
}

fn app_thread_entry(
    sender_to_dcs: TaskSender<Lua>,
    rx_from_gui: Receiver<AppMessage>,
    tx_to_gui: Sender<AppReply>,
) {
    loop {
        let mesg = rx_from_gui.recv();
        if mesg.is_err() {
            continue;
        }
        match mesg.as_ref().unwrap() {
            AppMessage::StartupAircraft => {
                let send_result = sender_to_dcs.send(|lua| start_jet(&lua)).wait();
                if let Err(e) = send_result {
                    log::warn!("Error {:?} sending start message to DCS", e);
                }
            }
            AppMessage::StopApp => return,
            AppMessage::InterruptAircraftStart => continue,
        };
    }
}

fn start_jet(lua: &Lua) {
    use dcs::mig21bis::Switch;
    let switches_to_start = [
        Switch::CanopyClose,
        Switch::FuelPump1,
        Switch::FuelPump3,
        Switch::FuelPumpDrain,
        Switch::BatteryOn,
        Switch::BatteryHeat,
        Switch::AcGenerator,
        Switch::DcGenerator,
        Switch::SprdPower,
        Switch::SprdDropPower,
        Switch::Po750Inverter1,
        Switch::Po750Inverter2,
        Switch::ApuPower,
        Switch::FireExtinguisherPower,
    ];

    for s in switches_to_start {
        let _ = dcs::mig21bis::set_switch(lua, s);
    }
}
