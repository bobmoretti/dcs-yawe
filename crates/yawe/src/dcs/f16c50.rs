#![allow(dead_code)]
#![allow(unused_variables)]
use crate::app::FsmMessage;
use crate::dcs::{self, list_indication, set_lockon_command, LockonCommand, SwitchInfo};
use mlua::prelude::LuaResult;
use mlua::Lua;
use offload::TaskSender;
use strum::IntoStaticStr;

use super::perform_click;

type Si = SwitchInfo<Switch>;
enum Info {
    Toggle(SwitchInfo<Switch>),
    MultiToggle(SwitchInfo<Switch>),
    Momentary(SwitchInfo<Switch>),
    SpringLoaded3Pos(SpringLoaded3PosInfo),
    DualCommand3Pos(DualCommand3PosInfo),
    FloatValue(SwitchInfo<Switch>),
    Axis(SwitchInfo<Switch>),
}

enum ThreePosState {
    Down,
    Stop,
    Up,
}

enum ThreePosToggleState {
    Down,
    Middle,
    Up,
}

struct SpringLoaded3PosInfo {
    pub info: Si,
    pub command_up: i32,
}

struct DualCommand3PosInfo {
    pub device_id: i32,
    pub command_up: i32,
    pub command_down: i32,
    pub argument: i32,
}

#[derive(Debug, Clone, Copy, IntoStaticStr)]
#[allow(dead_code)]
pub enum Switch {
    MainPower,
    Jfs,
    CanopyRetract,
    CanopyValue,
    CanopyLock,
    EngineTachometer,
    MmcPower,
    StoresStationPower,
    MfdPower,
    UfcPower,
    GpsPower,
    MapPower,
    DlPower,
    MidsLvtControl,
    LeftHardpointPower,
    RightHardpointPower,
    FcrPower,
    RadAltPower,
    IffMasterKnob,
    UhfFunctionKnob,
    CmdsPower,
    CmdsJammerPower,
    CmdsMwsPower,
    CmdsExpendable1Power,
    CmdsExpendable2Power,
    CmdsExpendable3Power,
    CmdsExpendable4Power,
    CmdsProgramKnob,
    CmdsModeKnob,
    HudBrightnessKnob,
    HmdIntensityKnob,
    LaserArm,
    RwrPower,
    SaiCage,
    SaiPitchTrim,
    AntiSkid,
    EjectionSafety,
    AltimeterModeLever,
    NumSwitches,
}

const SWITCH_INFO_MAP: [Info; Switch::NumSwitches as usize] = [
    Info::MultiToggle(Si::new(Switch::MainPower, 3, 3001, 510)),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::Jfs, 6, 3006, 447),
        command_up: (3005),
    }),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::CanopyRetract, 10, 3003, 606),
        command_up: 3002,
    }),
    Info::FloatValue(Si::new_float(Switch::CanopyValue, 7)),
    Info::MultiToggle(Si::new(Switch::CanopyLock, 10, 3004, 600)),
    Info::FloatValue(Si::new_float(Switch::EngineTachometer, 95)),
    Info::Toggle(Si::new(Switch::MmcPower, 19, 3001, 715)),
    Info::Toggle(Si::new(Switch::StoresStationPower, 22, 3001, 716)),
    Info::Toggle(Si::new(Switch::MfdPower, 19, 3014, 717)),
    Info::Toggle(Si::new(Switch::UfcPower, 17, 3001, 718)),
    Info::Toggle(Si::new(Switch::GpsPower, 59, 3001, 720)),
    Info::Toggle(Si::new(Switch::MapPower, 61, 3001, 722)),
    Info::Toggle(Si::new(Switch::DlPower, 60, 3001, 721)),
    Info::MultiToggle(Si::new(Switch::MidsLvtControl, 41, 3001, 723)),
    Info::Toggle(Si::new(Switch::LeftHardpointPower, 22, 3002, 670)),
    Info::Toggle(Si::new(Switch::RightHardpointPower, 22, 3003, 671)),
    Info::Toggle(Si::new(Switch::FcrPower, 31, 3001, 672)),
    Info::Toggle(Si::new(Switch::RadAltPower, 15, 3001, 673)),
    Info::MultiToggle(Si::new(Switch::IffMasterKnob, 35, 3002, 540)),
    Info::MultiToggle(Si::new(Switch::UhfFunctionKnob, 37, 3008, 417)),
    Info::Toggle(Si::new(Switch::CmdsPower, 32, 3001, 375)),
    Info::Toggle(Si::new(Switch::CmdsJammerPower, 32, 3002, 374)),
    Info::Toggle(Si::new(Switch::CmdsMwsPower, 32, 3003, 373)),
    Info::Toggle(Si::new(Switch::CmdsExpendable1Power, 32, 3005, 365)),
    Info::Toggle(Si::new(Switch::CmdsExpendable2Power, 32, 3006, 366)),
    Info::Toggle(Si::new(Switch::CmdsExpendable3Power, 32, 3007, 367)),
    Info::Toggle(Si::new(Switch::CmdsExpendable4Power, 32, 3008, 368)),
    Info::MultiToggle(Si::new(Switch::CmdsProgramKnob, 32, 3009, 377)),
    Info::MultiToggle(Si::new(Switch::CmdsModeKnob, 32, 3010, 378)),
    Info::Axis(Si::new(Switch::HudBrightnessKnob, 17, 3022, 190)),
    Info::Axis(Si::new(Switch::HmdIntensityKnob, 30, 3001, 392)),
    Info::Toggle(Si::new(Switch::LaserArm, 22, 3004, 103)),
    Info::Toggle(Si::new(Switch::RwrPower, 33, 3011, 401)),
    Info::Momentary(Si::new(Switch::SaiCage, 47, 3002, 67)),
    Info::Axis(Si::new(Switch::SaiPitchTrim, 47, 3003, 66)),
    Info::DualCommand3Pos(DualCommand3PosInfo {
        device_id: 7,
        command_up: 3010,
        command_down: 3004,
        argument: 357,
    }),
    Info::Toggle(Si::new(Switch::EjectionSafety, 10, 3009, 785)),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::AltimeterModeLever, 45, 3002, 60),
        command_up: 3001,
    }),
];

#[allow(unreachable_patterns)]
fn get_switch_info(s: Switch) -> Option<&'static Si> {
    let info = &SWITCH_INFO_MAP[s as usize];
    match info {
        Info::Toggle(i) => Some(i),
        Info::MultiToggle(i) => Some(i),
        Info::Momentary(i) => Some(i),
        Info::FloatValue(i) => Some(i),
        Info::SpringLoaded3Pos(i) => Some(&i.info),
        Info::Axis(i) => Some(i),
        Info::DualCommand3Pos(_) => None,
    }
}

fn toggle_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    let i = get_switch_info(s);
    if let Some(info) = i {
        dcs::perform_click(lua, info.device_id, info.command, 1.0)
    } else {
        log::warn!("Tried to toggle {:?} which is not possible", s);
        LuaResult::Ok(())
    }
}

pub fn set_switch_state(lua: &Lua, s: Switch, state: f32) -> LuaResult<()> {
    let i = get_switch_info(s);
    if let Some(info) = i {
        dcs::perform_click(lua, info.device_id, info.command, state)
    } else {
        log::warn!("Tried to set the state of {:?} which is not possible", s);
        LuaResult::Ok(())
    }
}

pub fn get_switch_state(lua: &Lua, s: Switch) -> LuaResult<f32> {
    if let Some(info) = get_switch_info(s) {
        dcs::get_switch_state(lua, 0, info.argument)
    } else {
        log::warn!("Tried to get the state of {:?} which is not possible", s);
        LuaResult::Ok(0.0)
    }
}

pub fn is_switch_set(lua: &Lua, s: Switch) -> LuaResult<bool> {
    Ok(get_switch_state(lua, s)? > 0.5)
}

pub fn set_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    if !is_switch_set(lua, s)? {
        toggle_switch(lua, s)
    } else {
        Ok(())
    }
}

pub fn unset_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    if is_switch_set(lua, s)? {
        toggle_switch(lua, s)
    } else {
        Ok(())
    }
}

fn set_three_pos_springloaded(lua: &Lua, s: Switch, state: ThreePosState) -> LuaResult<()> {
    let Info::SpringLoaded3Pos(three_pos_info) = &SWITCH_INFO_MAP[s as usize] else {
        log::warn!(
            "Tried to interpret {:?} as a three pos springloaded switch",
            s
        );
        return LuaResult::Ok(());
    };
    let device_id = three_pos_info.info.device_id;
    let down_command = three_pos_info.info.command;
    let up_command = three_pos_info.command_up;
    dcs::perform_click(lua, device_id, down_command, 0.0)?;
    dcs::perform_click(lua, device_id, up_command, 0.0)?;

    match state {
        ThreePosState::Down => dcs::perform_click(lua, device_id, down_command, -1.0),
        ThreePosState::Stop => LuaResult::Ok(()),
        ThreePosState::Up => dcs::perform_click(lua, device_id, up_command, 1.0),
    }?;

    Ok(())
}

fn set_three_pos(lua: &Lua, s: Switch, state: ThreePosToggleState) -> LuaResult<()> {
    let Info::DualCommand3Pos(info) = &SWITCH_INFO_MAP[s as usize] else {
        log::warn!(
            "Tried to interpret {:?} as a three pos springloaded switch, but it is not",
            s
        );
        return LuaResult::Ok(());
    };

    match state {
        ThreePosToggleState::Down => perform_click(lua, info.device_id, info.command_down, -1.0),
        ThreePosToggleState::Middle => {
            perform_click(lua, info.device_id, info.command_down, 0.0)?;
            perform_click(lua, info.device_id, info.command_up, -1.0)
        }
        ThreePosToggleState::Up => {
            perform_click(lua, info.device_id, info.command_down, 1.0)?;
            perform_click(lua, info.device_id, info.command_up, 0.0)
        }
    }
}

fn poll_argument(to_gamegui: &TaskSender<Lua>, switch: Switch) -> Result<f32, crate::Error> {
    to_gamegui
        .send(move |lua| get_switch_state(lua, switch))
        .wait()
        .map_err(|_| crate::Error::CommError)?
        .map_err(|e| crate::Error::LuaError(e))
}

fn handle_polling_err(r: &Result<f32, crate::Error>) {
    if let Err(err) = r {
        match err {
            crate::Error::LuaError(e) => {
                log::warn!(
                    "Lua error encountered when polling Engine Start Light {:?}",
                    e
                )
            }
            _ => {}
        }
    }
}

enum IndicationDevice {
    Ufc = 6,
}

fn get_avionics_indication(lua: &Lua, device: IndicationDevice) -> LuaResult<String> {
    list_indication(lua, device as i32)
}

fn throw_initial_switches(tx: &TaskSender<Lua>) {
    let _ = tx
        .send(|lua| {
            let switch_states = [
                (Switch::MmcPower, 1.0),
                (Switch::StoresStationPower, 1.0),
                (Switch::MfdPower, 1.0),
                (Switch::UfcPower, 1.0),
                (Switch::MapPower, 1.0),
                (Switch::GpsPower, 1.0),
                (Switch::DlPower, 1.0),
                (Switch::LeftHardpointPower, 1.0),
                (Switch::RightHardpointPower, 1.0),
                (Switch::FcrPower, 1.0),
                (Switch::RadAltPower, 1.0),
                (Switch::CmdsPower, 1.0),
                (Switch::CmdsJammerPower, 1.0),
                (Switch::CmdsMwsPower, 1.0),
                (Switch::CmdsExpendable1Power, 1.0),
                (Switch::CmdsExpendable2Power, 1.0),
                (Switch::CmdsExpendable3Power, 1.0),
                (Switch::CmdsExpendable4Power, 1.0),
                (Switch::MidsLvtControl, 0.2),
                (Switch::IffMasterKnob, 0.3),
                (Switch::UhfFunctionKnob, 0.2),
                (Switch::CmdsProgramKnob, 0.1),
                (Switch::CmdsModeKnob, 0.2),
                (Switch::HudBrightnessKnob, 1.0),
                (Switch::HmdIntensityKnob, 1.0),
                (Switch::LaserArm, 1.0),
                (Switch::RwrPower, 1.0),
                (Switch::EjectionSafety, 1.0),
            ];
            for (switch, state) in switch_states {
                let _ = set_switch_state(lua, switch, state);
            }
            let _ = set_three_pos(lua, Switch::AntiSkid, ThreePosToggleState::Middle);
        })
        .wait()
        .map_err(|_| crate::Error::CommError);
}

#[derive(Default, Debug, PartialEq, Clone)]
struct Timer {
    start_time: f32,
    expire: f32,
}

impl Timer {
    pub fn new(duration: f32, now: f32) -> Self {
        Self {
            start_time: now,
            expire: duration,
        }
    }

    pub fn is_expired(&self, now: f32) -> bool {
        now - self.start_time >= self.expire
    }
}

#[derive(Clone, PartialEq, Debug)]
enum StartupState {
    ColdDark,
    WaitCanopyClosed,
    WaitAfterCanopyClosed,
    WaitCanopySwitchRelease,
    WaitCanopyLocked,
    WaitJfsSpool,
    WaitGeneratorsRunning,
    WaitInsAligned,
    Done,
}

#[derive(Debug, Clone)]
pub struct Fsm {
    state: StartupState,
    to_gamegui: TaskSender<Lua>,
    to_export: TaskSender<Lua>,
    gui: crate::gui::TxHandle,
    sim_time: f32,
    canopy_timer: Timer,
}

impl dcs::AircraftFsm for Fsm {
    fn run_fsm(&mut self, event: FsmMessage, sim_time: f32) {
        self.sim_time = sim_time;
        match self.state {
            StartupState::ColdDark => self.cold_dark_handler(event),
            StartupState::WaitCanopyClosed => self.wait_canopy_closed(event),
            StartupState::WaitAfterCanopyClosed => self.wait_after_canopy_closed(event),
            StartupState::WaitCanopySwitchRelease => self.wait_canopy_switch_released(event),
            StartupState::WaitCanopyLocked => self.wait_canopy_locked(event),
            StartupState::WaitJfsSpool => self.wait_jfs_spool(event),
            StartupState::WaitGeneratorsRunning => self.wait_generators_running(event),
            StartupState::Done => self.done(event),
            StartupState::WaitInsAligned => todo!(),
        }
    }
}

impl Fsm {
    pub fn new(
        to_gamegui: TaskSender<Lua>,
        to_export: TaskSender<Lua>,
        gui: crate::gui::TxHandle,
    ) -> Self {
        Self {
            state: StartupState::ColdDark,
            to_gamegui,
            to_export,
            gui,
            sim_time: 0.0,
            canopy_timer: Timer::default(),
        }
    }
    fn cold_dark_handler(&mut self, event: crate::app::FsmMessage) {
        match event {
            crate::app::FsmMessage::StartupAircraft => {
                // this should cause the progress bar to begin animating
                self.gui.set_startup_progress(0.001);
                self.gui.set_startup_text("Setting up initial switches");
                self.gui.set_startup_progress(0.05);
                self.gui.set_startup_text("Waiting for canopy to close");
                self.to_gamegui.send(|lua| {
                    let _ = set_switch_state(lua, Switch::MainPower, 1.0);
                    let _ =
                        set_three_pos_springloaded(lua, Switch::CanopyRetract, ThreePosState::Down);
                    let _ = set_three_pos_springloaded(lua, Switch::Jfs, ThreePosState::Down);
                });
                throw_initial_switches(&self.to_gamegui);
                self.state = StartupState::WaitCanopyClosed;
            }
            _ => {}
        }
    }

    fn wait_canopy_closed(&mut self, event: crate::app::FsmMessage) {
        let Ok(lua_result) = self
            .to_gamegui
            .send(|lua| get_switch_state(lua, Switch::CanopyValue))
            .wait()
        else {
            return;
        };

        let Ok(canopy_position) = lua_result else {
            log::warn!("Polling canopy position failed!");
            return;
        };

        if canopy_position != 0.0 {
            return;
        }

        // The F-16 continues to play a sound for a few seconds after the canopy state
        // is reported at 0, and the aircraft seems to have a hidden/internal state that
        // keeps track of how far "after fully closed" the canopy can be. Use a timer to
        // continue holding down the canopy close switch for a few more seconds.
        self.canopy_timer = Timer::new(2.3, self.sim_time);
        self.gui
            .set_startup_text("Waiting for canopy to fully close");

        self.state = StartupState::WaitAfterCanopyClosed;
    }

    fn wait_after_canopy_closed(&mut self, event: crate::app::FsmMessage) {
        if !self.canopy_timer.is_expired(self.sim_time) {
            return;
        }

        self.gui.set_startup_progress(0.1);
        self.gui.set_startup_text("Releasing canopy close switch");
        let _ = self
            .to_gamegui
            .send(|lua| {
                let _ = set_three_pos_springloaded(lua, Switch::CanopyRetract, ThreePosState::Stop);
            })
            .wait();

        self.state = StartupState::WaitCanopySwitchRelease;
    }

    fn wait_canopy_switch_released(&mut self, event: crate::app::FsmMessage) {
        let Ok(lua_result) = self
            .to_gamegui
            .send(|lua| get_switch_state(lua, Switch::CanopyRetract))
            .wait()
        else {
            return;
        };

        let Ok(canopy_switch_state) = lua_result else {
            log::warn!("Polling canopy switch position failed!");
            return;
        };
        log::debug!("Canopy switch state: {canopy_switch_state}");

        if canopy_switch_state < 0.0 {
            return;
        }

        self.to_gamegui.send(|lua| {
            let _ = set_switch_state(lua, Switch::CanopyLock, 1.0);
        });
        self.gui.set_startup_text("Locking canopy");

        self.state = StartupState::WaitCanopyLocked;
    }

    fn wait_canopy_locked(&mut self, event: crate::app::FsmMessage) {
        let Ok(lua_result) = self
            .to_gamegui
            .send(|lua| get_switch_state(lua, Switch::CanopyLock))
            .wait()
        else {
            return;
        };

        let Ok(canopy_lock_lever_state) = lua_result else {
            log::warn!("Couldn't read canopy lever state");
            return;
        };

        if canopy_lock_lever_state < 1.0 {
            return;
        }
        self.gui.set_startup_text("Waiting for JFS");
        self.state = StartupState::WaitJfsSpool;
    }

    fn wait_jfs_spool(&mut self, event: crate::app::FsmMessage) {
        let Ok(lua_result) = self
            .to_gamegui
            .send(|lua| get_switch_state(lua, Switch::EngineTachometer))
            .wait()
        else {
            return;
        };

        let Ok(engine_rpm_normalized) = lua_result else {
            log::warn!("Couldn't read engine RPM");
            return;
        };

        const ENGINE_START_THRESHOLD: f32 = 0.12;
        if engine_rpm_normalized < ENGINE_START_THRESHOLD {
            return;
        }

        self.gui.set_startup_text("Waiting for engine to spool");
        let _ = self
            .to_gamegui
            .send(|lua| set_lockon_command(lua, LockonCommand::LeftEngineStart))
            .wait();

        let _ = self
            .to_gamegui
            .send(|lua| {
                let _ = set_switch_state(lua, Switch::SaiCage, -1.0);
                let _ = set_switch_state(lua, Switch::SaiPitchTrim, 0.504);
            })
            .wait();
        let _ = self.to_gamegui.send(|lua| {
            let _ = set_switch_state(lua, Switch::SaiCage, 0.0);
            let _ =
                set_three_pos_springloaded(lua, Switch::AltimeterModeLever, ThreePosState::Down);
        });

        self.state = StartupState::WaitGeneratorsRunning;
    }

    fn wait_generators_running(&mut self, event: crate::app::FsmMessage) {
        let Ok(lua_result) = self
            .to_export
            .send(|lua| get_avionics_indication(lua, IndicationDevice::Ufc))
            .wait()
        else {
            return;
        };
        let Ok(indication) = lua_result else {
            log::warn!("Couldn't get list indication when waiting for engine start");
            return;
        };
        if indication.is_empty() {
            return;
        }

        self.to_gamegui.send(|lua| {
            let _ =
                set_three_pos_springloaded(lua, Switch::AltimeterModeLever, ThreePosState::Stop);
        });
        self.state = StartupState::Done;
    }
    fn done(&self, event: crate::app::FsmMessage) {}
}
