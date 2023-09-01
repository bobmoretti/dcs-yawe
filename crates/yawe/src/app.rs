use crate::dcs;
use crate::gui;
use mlua::Lua;
use offload::{PackagedTask, TaskSender};
use rsevents::Awaitable;
use rsevents::{AutoResetEvent, EventState};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;

pub struct App {
    thread: Option<JoinHandle<()>>,
    tx_to_app: Sender<AppMessage>,
    gui: gui::Handle,
    ownship_type: dcs::AircraftId,
    rx_from_dcs_gamegui: Receiver<PackagedTask<Lua>>,
    rx_from_dcs_export: Receiver<PackagedTask<Lua>>,
}

static AWAKEN_APP_THREAD: AutoResetEvent = AutoResetEvent::new(EventState::Unset);

#[derive(Clone, Copy, PartialEq)]
pub enum AppMessage {
    StartupAircraft,
    InterruptAircraftStart,
    StopApp,
    None,
}

pub enum AppReply {
    StartupComplete,
}

impl App {
    // Start the main application (scoped) thread, return an interface handle to
    // allow the outside world to talk to it.
    pub fn new() -> Self {
        //
        let (tx_to_dcs_gamegui, rx_from_dcs_gamegui) = TaskSender::new();
        let (tx_to_dcs_export, rx_from_dcs_export) = TaskSender::new();
        let (tx_to_app, rx_from_gui) = channel::<AppMessage>();
        let (tx_to_gui, rx_from_app) = channel::<AppReply>();

        let gui = gui::Handle::new(
            tx_to_app.clone(),
            tx_to_dcs_gamegui.clone(),
            tx_to_dcs_export.clone(),
        );

        let thread = std::thread::spawn(|| {
            app_thread_entry(tx_to_dcs_gamegui, tx_to_dcs_export, rx_from_gui, tx_to_gui)
        });

        let me = Self {
            thread: Some(thread),
            tx_to_app: tx_to_app.clone(),
            gui: gui,
            ownship_type: dcs::AircraftId::Unknown(String::from("")),
            rx_from_dcs_export,
            rx_from_dcs_gamegui,
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
        AWAKEN_APP_THREAD.set();
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

        // todo: implement timeout, but for now just process all pending messages ASAP.
        while let Ok(_) = self.rx_from_dcs_gamegui.try_recv().map(|job| job(lua)) {}
        AWAKEN_APP_THREAD.set();

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

        // todo: implement timeout, but for now just process all pending messages ASAP.
        while let Ok(_) = self.rx_from_dcs_export.try_recv().map(|job| job(lua)) {}
        0
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn app_thread_entry(
    sender_to_dcs_gamegui: TaskSender<Lua>,
    sender_to_dcs_export: TaskSender<Lua>,
    rx_from_gui: Receiver<AppMessage>,
    tx_to_gui: Sender<AppReply>,
) {
    let mut fsm = dcs::mig21bis::Fsm::new(sender_to_dcs_gamegui, sender_to_dcs_export);
    loop {
        AWAKEN_APP_THREAD.wait();
        if let Ok(msg) = rx_from_gui.try_recv() {
            if msg == AppMessage::StopApp {
                return;
            }
            fsm.run(msg);
        } else {
            fsm.run(AppMessage::None);
        }
    }
}
