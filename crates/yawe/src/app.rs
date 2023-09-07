use crate::dcs;
use crate::dcs::AircraftFsm;
use crate::gui;
use mlua::Lua;
use offload::{PackagedTask, TaskSender};
use rsevents::Awaitable;
use rsevents::{AutoResetEvent, EventState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;

static AWAKEN_APP_THREAD: AutoResetEvent = AutoResetEvent::new(EventState::Unset);
static STOP_APP_THREAD: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FsmMessage {
    StartupAircraft,
    _InterruptAircraftStart,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMessage {
    AircraftChanged(dcs::AircraftId),
    FsmEvent(FsmMessage),
}

pub struct App {
    thread: Option<JoinHandle<()>>,
    _tx_to_app: Sender<AppMessage>,
    gui: gui::Handle,
    ownship_type: dcs::AircraftId,
    rx_from_dcs_gamegui: Option<Receiver<PackagedTask<Lua>>>,
    rx_from_dcs_export: Option<Receiver<PackagedTask<Lua>>>,
    paused: bool,
}

impl App {
    // Start the main application (scoped) thread, return an interface handle to
    // allow the outside world to talk to it.
    pub fn new() -> Self {
        //
        let (tx_to_dcs_gamegui, rx_from_dcs_gamegui) = TaskSender::new();
        let (tx_to_dcs_export, rx_from_dcs_export) = TaskSender::new();
        let (tx_to_app, rx_from_gui) = channel::<AppMessage>();

        let gui = gui::Handle::new(
            tx_to_app.clone(),
            tx_to_dcs_gamegui.clone(),
            tx_to_dcs_export.clone(),
        );

        let handle = gui.tx_handle();

        let thread = std::thread::Builder::new()
            .name("yawe-app".to_string())
            .spawn(move || {
                app_thread_entry(tx_to_dcs_gamegui, tx_to_dcs_export, handle, rx_from_gui)
            })
            .unwrap();

        let me = Self {
            thread: Some(thread),
            _tx_to_app: tx_to_app.clone(),
            gui: gui,
            ownship_type: dcs::AircraftId::Unknown(String::from("")),
            rx_from_dcs_gamegui: Some(rx_from_dcs_gamegui),
            rx_from_dcs_export: Some(rx_from_dcs_export),
            paused: false,
        };
        me
    }

    fn set_paused(&mut self) {
        self.paused = true;
        self.gui.set_paused();
    }

    fn set_unpaused(&mut self) {
        if !self.paused {
            return;
        }
        self.paused = false;
        self.gui.set_unpaused();
    }

    pub fn on_start(&mut self, lua: &Lua) -> i32 {
        if dcs::is_paused(lua).unwrap_or(false) {
            log::info!("Game is starting paused");
            self.set_paused();
        }
        0
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
                self.gui.set_ownship_type(ownship_type.clone());
            }
            let _ = self
                ._tx_to_app
                .send(AppMessage::AircraftChanged(ownship_type));
        }

        // todo: implement timeout, but for now just process all pending messages ASAP.
        while let Ok(_) = self
            .rx_from_dcs_gamegui
            .as_ref()
            .unwrap()
            .try_recv()
            .map(|job| job(lua))
        {}
        AWAKEN_APP_THREAD.set();

        if self.gui.is_running() {
            0
        } else {
            -1
        }
    }

    pub fn on_frame_export(&mut self, lua: &Lua) -> i32 {
        // todo: implement timeout, but for now just process all pending messages ASAP.
        while let Ok(_) = self
            .rx_from_dcs_export
            .as_ref()
            .unwrap()
            .try_recv()
            .map(|job| job(lua))
        {}
        0
    }

    pub fn on_simulation_pause(&mut self, _lua: &Lua) -> i32 {
        log::info!("Simulation paused");
        self.set_paused();
        0
    }
    pub fn on_simulation_resume(&mut self, _lua: &Lua) -> i32 {
        log::info!("Simulation resumed");
        self.set_unpaused();
        0
    }

    pub fn stop(&mut self) {
        log::info!("Signaling app thread to stop");
        self.rx_from_dcs_gamegui = None;
        self.rx_from_dcs_export = None;
        STOP_APP_THREAD.store(true, Ordering::SeqCst);
        AWAKEN_APP_THREAD.set();
        let thread_finish = std::mem::take(&mut self.thread);
        self.gui.stop();
        log::info!("joining on thread finishing");
        thread_finish.unwrap().join().unwrap();
        log::info!("App thread stopped");
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
    gui_handle: gui::TxHandle,
    rx_from_gui: Receiver<AppMessage>,
) {
    // need to dispatch between several
    let mut fsm: Box<dyn AircraftFsm> = Box::new(dcs::EmptyFsm::new(
        sender_to_dcs_gamegui.clone(),
        sender_to_dcs_export.clone(),
        gui_handle.clone(),
    ));

    loop {
        AWAKEN_APP_THREAD.wait();

        if STOP_APP_THREAD.load(Ordering::SeqCst) {
            log::info!("stopping app thread!");
            return;
        }
        match rx_from_gui.try_recv() {
            Ok(msg) => match msg {
                AppMessage::AircraftChanged(aircraft) => {
                    fsm = dcs::get_fsm(
                        aircraft,
                        sender_to_dcs_gamegui.clone(),
                        sender_to_dcs_export.clone(),
                        gui_handle.clone(),
                    )
                }
                AppMessage::FsmEvent(fsm_msg) => fsm.run(fsm_msg),
            },
            Err(e) => {
                if let TryRecvError::Disconnected = e {
                    log::info!("stopping app thread via disconnected");
                    return;
                } else {
                    fsm.run(FsmMessage::None);
                }
            }
        }
    }
}
