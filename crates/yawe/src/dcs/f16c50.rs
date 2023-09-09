#![allow(dead_code)]
#![allow(unused_variables)]
use super::SwitchInfo;
use crate::app::FsmMessage;
use crate::dcs;
use mlua::prelude::LuaResult;
use mlua::Lua;
use offload::TaskSender;
use strum::IntoStaticStr;

#[derive(Debug, Clone, Copy, IntoStaticStr)]
#[allow(dead_code)]
pub enum Switch {
    MainPower,
    Jfs,
    CanopyRetract,
    NumSwitches,
}

type Si = SwitchInfo<Switch>;
pub enum Info {
    MultiToggle(SwitchInfo<Switch>),
    Momentary(SwitchInfo<Switch>),
    SpringLoaded3Pos(SpringLoaded3PosInfo),
}

enum ThreePosState {
    Down,
    Stop,
    Up,
}

pub struct SpringLoaded3PosInfo {
    pub info: Si,
    pub command_up: i32,
}

pub const SWITCH_INFO_MAP: [Info; Switch::NumSwitches as usize] = [
    Info::MultiToggle(Si::new(Switch::MainPower, 3, 3001, 510)),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::Jfs, 6, 3006, 447),
        command_up: (3005),
    }),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::CanopyRetract, 10, 3003, 606),
        command_up: 3002,
    }),
];

#[derive(Clone, PartialEq, Debug)]
enum StartupState {
    ColdDark,
    WaitCanopyClosed,
    WaitEngineStartBegun,
    WaitEngineStartComplete,
    WaitInsAligned,
    Done,
}

#[allow(unreachable_patterns)]
fn get_switch_info(s: Switch) -> Option<&'static Si> {
    let info = &SWITCH_INFO_MAP[s as usize];
    match info {
        Info::MultiToggle(i) => Some(i),
        Info::Momentary(i) => Some(i),
        Info::SpringLoaded3Pos(i) => Some(&i.info),
        _ => None,
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

#[derive(Debug, Clone)]
pub struct Fsm {
    state: StartupState,
    to_dcs_gamegui: TaskSender<Lua>,
    to_dcs_export: TaskSender<Lua>,
    gui: crate::gui::TxHandle,
}

impl dcs::AircraftFsm for Fsm {
    fn run_fsm(&mut self, event: FsmMessage) {
        match self.state {
            StartupState::ColdDark => self.cold_dark_handler(event),
            StartupState::WaitCanopyClosed => self.wait_canopy_closed(event),
            StartupState::WaitEngineStartBegun => self.wait_engine_start_begun(event),
            StartupState::WaitEngineStartComplete => self.wait_engine_start_complete(event),
            StartupState::Done => self.done(event),
            StartupState::WaitInsAligned => todo!(),
        }
    }
}

impl Fsm {
    pub fn new(
        to_dcs_gamegui: TaskSender<Lua>,
        to_dcs_export: TaskSender<Lua>,
        gui: crate::gui::TxHandle,
    ) -> Self {
        Self {
            state: StartupState::ColdDark,
            to_dcs_gamegui,
            to_dcs_export,
            gui,
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
                self.to_dcs_gamegui.send(|lua| {
                    let _ = set_switch_state(lua, Switch::MainPower, 1.0);
                    let _ =
                        set_three_pos_springloaded(lua, Switch::CanopyRetract, ThreePosState::Down);
                    let _ = set_three_pos_springloaded(lua, Switch::Jfs, ThreePosState::Down);
                });
                self.state = StartupState::WaitCanopyClosed;
            }
            _ => {}
        }
    }

    fn wait_canopy_closed(&self, event: crate::app::FsmMessage) {}
    fn wait_engine_start_begun(&self, event: crate::app::FsmMessage) {}
    fn wait_engine_start_complete(&self, event: crate::app::FsmMessage) {}
    fn done(&self, event: crate::app::FsmMessage) {}
}
