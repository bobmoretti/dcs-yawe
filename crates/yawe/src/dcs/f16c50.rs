#![allow(dead_code)]
#![allow(unused_variables)]

use crate::app::FsmMessage;
use crate::dcs::{self, list_indication, set_lockon_command, LockonCommand, SwitchInfo};
use egui_backend::egui;
use mlua::prelude::LuaResult;
use mlua::Lua;
use offload::TaskSender;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use strum::IntoStaticStr;
use trace::trace;
trace::init_depth_var!();

use super::{lookup_tree, perform_click, IndicationNode};

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum ThreePosState {
    Down,
    Stop,
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    InsKnob,
    Icp1,
    Icp2,
    Icp3,
    Icp4,
    Icp5,
    Icp6,
    Icp7,
    Icp8,
    Icp9,
    Icp0,
    IcpCom1,
    IcpCom2,
    IcpIff,
    IcpList,
    IcpAaMode,
    IcpAgMode,
    IcpRcl,
    IcpEnter,
    IcpDedInc,
    IcpDataRtnSeq,
    IcpDataUpDown,
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
    Info::MultiToggle(Si::new(Switch::InsKnob, 14, 3001, 719)),
    Info::Momentary(Si::new(Switch::Icp1, 17, 3003, 171)),
    Info::Momentary(Si::new(Switch::Icp2, 17, 3004, 172)),
    Info::Momentary(Si::new(Switch::Icp3, 17, 3005, 173)),
    Info::Momentary(Si::new(Switch::Icp4, 17, 3006, 175)),
    Info::Momentary(Si::new(Switch::Icp5, 17, 3007, 176)),
    Info::Momentary(Si::new(Switch::Icp6, 17, 3008, 177)),
    Info::Momentary(Si::new(Switch::Icp7, 17, 3009, 179)),
    Info::Momentary(Si::new(Switch::Icp8, 17, 3010, 180)),
    Info::Momentary(Si::new(Switch::Icp9, 17, 3011, 181)),
    Info::Momentary(Si::new(Switch::Icp0, 17, 3002, 182)),
    Info::Momentary(Si::new(Switch::IcpCom1, 17, 3012, 165)),
    Info::Momentary(Si::new(Switch::IcpCom2, 17, 3013, 166)),
    Info::Momentary(Si::new(Switch::IcpIff, 17, 3014, 167)),
    Info::Momentary(Si::new(Switch::IcpList, 17, 3015, 168)),
    Info::Momentary(Si::new(Switch::IcpAaMode, 17, 3018, 169)),
    Info::Momentary(Si::new(Switch::IcpAgMode, 17, 3019, 170)),
    Info::Momentary(Si::new(Switch::IcpRcl, 17, 3017, 174)),
    Info::Momentary(Si::new(Switch::IcpEnter, 17, 3016, 178)),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::IcpDedInc, 17, 3031, 183),
        command_up: 3030,
    }),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::IcpDataRtnSeq, 17, 3032, 184),
        command_up: 3033,
    }),
    Info::SpringLoaded3Pos(SpringLoaded3PosInfo {
        info: Si::new(Switch::IcpDataUpDown, 17, 3035, 185),
        command_up: 3034,
    }),
];

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

fn get_switch_argument(s: Switch) -> i32 {
    let info = &SWITCH_INFO_MAP[s as usize];
    match info {
        Info::Toggle(i) => i.argument,
        Info::MultiToggle(i) => i.argument,
        Info::Momentary(i) => i.argument,
        Info::SpringLoaded3Pos(i) => i.info.argument,
        Info::DualCommand3Pos(i) => i.argument,
        Info::FloatValue(i) => i.argument,
        Info::Axis(i) => i.argument,
    }
}

#[trace(logging)]
fn toggle_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    let i = get_switch_info(s);
    if let Some(info) = i {
        dcs::perform_click(lua, info.device_id, info.command, 1.0)
    } else {
        log::warn!("Tried to toggle {:?} which is not possible", s);
        LuaResult::Ok(())
    }
}

#[trace(logging)]
pub fn set_switch_state(lua: &Lua, s: Switch, state: f32) -> LuaResult<()> {
    let i = get_switch_info(s);
    if let Some(info) = i {
        dcs::perform_click(lua, info.device_id, info.command, state)
    } else {
        log::warn!("Tried to set the state of {:?} which is not possible", s);
        LuaResult::Ok(())
    }
}

#[trace(logging)]
pub fn get_switch_state(lua: &Lua, s: Switch) -> LuaResult<f32> {
    let argument = get_switch_argument(s);
    dcs::get_switch_state(lua, 0, argument)
}

fn wait_switch_state(to_gamegui: &TaskSender<Lua>, s: Switch, value: f32) {
    loop {
        let Ok(lua_result) = to_gamegui.send(move |lua| get_switch_state(lua, s)).wait() else {
            return;
        };

        let Ok(switch_state) = lua_result else {
            log::warn!("Polling {s:?} position failed!");
            return;
        };

        if switch_state == value {
            return;
        }
    }
}

#[trace(logging)]
fn set_switch_and_wait(to_gamegui: &TaskSender<Lua>, s: Switch, value: f32) {
    let _ = to_gamegui
        .send(move |lua| set_switch_state(lua, s, value))
        .wait();
    wait_switch_state(to_gamegui, s, value)
}

#[trace(logging)]
fn actuate_momentary(to_gamegui: &TaskSender<Lua>, s: Switch, value: f32) {
    set_switch_and_wait(to_gamegui, s, value);
    set_switch_and_wait(to_gamegui, s, 0.0);
}

#[trace(logging)]
fn actuate_3pos_spring(to_gamegui: &TaskSender<Lua>, s: Switch, state: ThreePosState) {
    let _ = to_gamegui
        .send(move |lua| set_three_pos_springloaded(lua, s, state))
        .wait();
    wait_switch_state(
        to_gamegui,
        s,
        match state {
            ThreePosState::Down => -1.0,
            ThreePosState::Stop => 0.0,
            ThreePosState::Up => 1.0,
        },
    );
    wait_frame(to_gamegui);
    wait_frame(to_gamegui);
    let _ = to_gamegui
        .send(move |lua| set_three_pos_springloaded(lua, s, ThreePosState::Stop))
        .wait();
    wait_switch_state(to_gamegui, s, 0.0);
    wait_frame(to_gamegui);
    wait_frame(to_gamegui);
}

#[trace(logging)]
fn ded_return(to_gamegui: &TaskSender<Lua>) {
    actuate_3pos_spring(to_gamegui, Switch::IcpDataRtnSeq, ThreePosState::Down);
}

#[trace(logging)]
fn ded_sequence(to_gamegui: &TaskSender<Lua>) {
    actuate_3pos_spring(to_gamegui, Switch::IcpDataRtnSeq, ThreePosState::Up);
}

#[trace(logging)]
fn ded_down(to_gamegui: &TaskSender<Lua>) {
    actuate_momentary(to_gamegui, Switch::IcpDataUpDown, -1.0);
}

#[trace(logging)]
fn ded_up(to_gamegui: &TaskSender<Lua>) {
    actuate_momentary(to_gamegui, Switch::IcpDataUpDown, 1.0);
}

#[trace(logging)]
fn icp_list(to_gamegui: &TaskSender<Lua>) {
    actuate_momentary(to_gamegui, Switch::IcpList, 1.0);
}

#[trace(logging)]
fn ded_rocker_up(to_gamegui: &TaskSender<Lua>) {
    actuate_3pos_spring(to_gamegui, Switch::IcpDedInc, ThreePosState::Up);
}

#[trace(logging)]
fn ded_rocker_down(to_gamegui: &TaskSender<Lua>) {
    actuate_3pos_spring(to_gamegui, Switch::IcpDedInc, ThreePosState::Down);
}

#[trace(logging)]
fn wait_frame(to_gamegui: &TaskSender<Lua>) {
    let _ = to_gamegui.send(|_| {}).wait();
}

#[trace(logging)]
fn icp_number(to_gamegui: &TaskSender<Lua>, number: i32) {
    let switch = match number {
        0 => Switch::Icp0,
        1 => Switch::Icp1,
        2 => Switch::Icp2,
        3 => Switch::Icp3,
        4 => Switch::Icp4,
        5 => Switch::Icp5,
        6 => Switch::Icp6,
        7 => Switch::Icp7,
        8 => Switch::Icp8,
        9 => Switch::Icp9,
        _ => return,
    };
    actuate_momentary(to_gamegui, switch, 1.0);
}

#[trace(logging)]
pub fn is_switch_set(lua: &Lua, s: Switch) -> LuaResult<bool> {
    Ok(get_switch_state(lua, s)? > 0.5)
}

#[trace(logging)]
pub fn set_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    if !is_switch_set(lua, s)? {
        toggle_switch(lua, s)
    } else {
        Ok(())
    }
}

#[trace(logging)]
pub fn unset_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    if is_switch_set(lua, s)? {
        toggle_switch(lua, s)
    } else {
        Ok(())
    }
}

#[trace(logging)]
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

#[trace(logging)]
#[trace(logging)]
fn set_three_pos(
    to_gamegui: &TaskSender<Lua>,
    s: Switch,
    state: ThreePosToggleState,
) -> LuaResult<()> {
    let Info::DualCommand3Pos(info) = &SWITCH_INFO_MAP[s as usize] else {
        log::warn!(
            "Tried to interpret {:?} as a three pos springloaded switch, but it is not",
            s
        );
        return LuaResult::Ok(());
    };

    let Ok(lua_result) = to_gamegui
        .send(move |lua| match state {
            ThreePosToggleState::Down => {
                perform_click(lua, info.device_id, info.command_down, -1.0)
            }
            ThreePosToggleState::Middle => {
                perform_click(lua, info.device_id, info.command_down, 0.0)?;
                perform_click(lua, info.device_id, info.command_up, -1.0)
            }
            ThreePosToggleState::Up => perform_click(lua, info.device_id, info.command_up, 1.0),
        })
        .wait()
    else {
        return Ok(());
    };

    // If we don't add this hack, the Jet won't allow the switch to properly spring
    // back to center
    std::thread::sleep(std::time::Duration::from_millis(100));

    if state == ThreePosToggleState::Up {
        let _ = to_gamegui
            .send(|lua| perform_click(lua, info.device_id, info.command_up, 0.0))
            .wait()
            .unwrap_or(Ok(()));
    };

    Ok(())
}

#[trace(logging)]
fn poll_argument(to_gamegui: &TaskSender<Lua>, switch: Switch) -> Result<f32, crate::Error> {
    to_gamegui
        .send(move |lua| get_switch_state(lua, switch))
        .wait()
        .map_err(|_| crate::Error::CommError)?
        .map_err(|e| crate::Error::LuaError(e))
}

#[trace(logging)]
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

#[derive(Default, Debug, PartialEq, Clone)]
struct CmdsBingo {
    chaff: i8,
    flare: i8,
    feedback: bool,
    reqctr: bool,
    bingo: bool,
}

#[derive(Default, Debug, PartialEq, Clone)]
struct CmdsProgramSlot {
    burst_quantity: i8,
    burst_interval: f32,
    sequence_quantity: i8,
    sequence_interval: f32,
}

#[derive(Default, Debug, PartialEq, Clone)]
struct CmdsProgram {
    chaff: CmdsProgramSlot,
    flare: CmdsProgramSlot,
}

#[derive(Default, Debug, PartialEq, Clone)]
struct Cmds {
    bingo: CmdsBingo,
    programs: [CmdsProgram; 6],
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct AvionicsState {
    cmds: Cmds,
}

#[trace(logging, disable(tree))]
fn parse_quantity<T>(tree: &slab_tree::Tree<IndicationNode>, path: &Vec<&str>) -> Option<T>
where
    T: std::str::FromStr + std::fmt::Debug,
{
    let value = &lookup_tree(tree, path)?.value;
    let parse_result = value.trim().parse::<T>();
    if let Err(e) = parse_result {
        log::error!("Error parsing path {path:?}, value was {value}");
        return None;
    };
    parse_result.ok()
}

#[trace(logging, disable(tree))]
fn parse_bool(tree: &slab_tree::Tree<IndicationNode>, path: &Vec<&str>) -> Option<bool> {
    match lookup_tree(tree, path)?.value.as_str() {
        "ON" => Some(true),
        "OFF" => Some(false),
        _ => None,
    }
}

#[trace(logging, pretty)]
fn read_cmds_bingo_page(to_export: &TaskSender<Lua>) -> Option<CmdsBingo> {
    let _ = super::get_avionics_indication(to_export, IndicationDevice::Ded as i32)?;
    let ded_indication = super::get_avionics_indication(to_export, IndicationDevice::Ded as i32)?;

    let chaff_count: i8 = parse_quantity(
        &ded_indication,
        &vec!["CMDS_CH_Scratchpad_placeholder", "CMDS_CH_Scratchpad"],
    )?;

    let flare_count: i8 = parse_quantity(
        &ded_indication,
        &vec!["CMDS_FL_Scratchpad_placeholder", "CMDS_FL_Scratchpad"],
    )?;

    Some(CmdsBingo {
        chaff: chaff_count,
        flare: flare_count,
        feedback: parse_bool(
            &ded_indication,
            &vec!["CMDS_FDBK_value_placeholder", "CMDS_FDBK_value"],
        )?,
        reqctr: parse_bool(
            &ded_indication,
            &vec!["CMDS_REQCTR_value_placeholder", "CMDS_REQCTR_value"],
        )?,
        bingo: parse_bool(
            &ded_indication,
            &vec!["CMDS_BINGO_value_placeholder", "CMDS_BINGO_value"],
        )?,
    })
}

#[trace(logging, disable(tree))]
fn parse_cmds_program_page(tree: &slab_tree::Tree<IndicationNode>) -> Option<CmdsProgramSlot> {
    if lookup_tree(tree, &vec!["CMDS_Prog_label"]).is_none() {
        log::warn!("Not on CMDS program page!");
        return None;
    }

    let bq: i8 = parse_quantity(
        tree,
        &vec!["CMDS_BQ_Scratchpad_placeholder", "CMDS_BQ_Scratchpad"],
    )?;

    let bi: f32 = parse_quantity(
        tree,
        &vec!["CMDS_BI_Scratchpad_placeholder", "CMDS_BI_Scratchpad"],
    )?;

    let sq: i8 = parse_quantity(
        tree,
        &vec!["CMDS_SQ_Scratchpad_placeholder", "CMDS_SQ_Scratchpad"],
    )?;

    let si: f32 = parse_quantity(
        tree,
        &vec!["CMDS_SI_Scratchpad_placeholder", "CMDS_SI_Scratchpad"],
    )?;

    Some(CmdsProgramSlot {
        burst_quantity: bq,
        burst_interval: bi,
        sequence_quantity: sq,
        sequence_interval: si,
    })
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum AvionicsError {
    InvalidState,
}

#[trace(logging)]
fn wait_on_cmds_program(to_export: &TaskSender<Lua>) {
    while (get_avionics_value(to_export, IndicationDevice::Ded, &vec!["CMDS_Prog_label"])).is_none()
    {
        log::info!("waiting on cmds program");
    }
}

// Ensures that the DED is in the CMDS menu, on program 1, and with the chaff
// bucket selected
#[trace(logging)]
fn get_to_cmds_program_root(
    to_gamegui: &TaskSender<Lua>,
    to_export: &TaskSender<Lua>,
) -> Result<(), AvionicsError> {
    loop {
        let tree = super::get_avionics_indication(to_export, IndicationDevice::Ded as i32)
            .ok_or(AvionicsError::InvalidState)?;
        let Some((kind, program)) = get_cmds_program(&tree) else {
            return Err(AvionicsError::InvalidState);
        };
        if (kind, program) == (Countermeasure::Chaff, 1) {
            return Ok(());
        }
        if kind != Countermeasure::Chaff {
            ded_sequence(to_gamegui);
        }
        if program > 1 {
            ded_rocker_down(to_gamegui);
        }
    }
}

#[trace(logging, pretty)]
fn read_cmds(to_gamegui: &TaskSender<Lua>, to_export: &TaskSender<Lua>) -> Option<AvionicsState> {
    while !is_on_cni(to_export) {
        ded_return(to_gamegui);
    }
    icp_list(to_gamegui);
    while !is_on_list(to_export) {}
    icp_number(to_gamegui, 7);
    while !is_on_cmds_bingo(to_export) {}
    let mut avionics = AvionicsState::default();
    avionics.cmds.bingo = read_cmds_bingo_page(to_export)?;
    ded_sequence(to_gamegui);
    wait_frame(to_gamegui);
    wait_on_cmds_program(to_export);
    get_to_cmds_program_root(to_gamegui, to_export).ok()?;

    for ii in 0..6 {
        let tree = super::get_avionics_indication(to_export, IndicationDevice::Ded as i32)?;
        let (kind, program) = get_cmds_program(&tree)?;
        if (Countermeasure::Chaff, ii + 1) != (kind, program) {
            log::error!(
                "Was not on correct page, expected {:?}, {}, got {:?}, {}",
                Countermeasure::Chaff,
                ii + 1,
                kind,
                program
            );
            return None;
        }
        avionics.cmds.programs[ii as usize].chaff = parse_cmds_program_page(&tree)?;
        ded_rocker_up(to_gamegui);
        wait_frame(to_gamegui);
    }

    ded_sequence(to_gamegui);
    wait_frame(to_gamegui);
    for ii in 0..6 {
        let tree = super::get_avionics_indication(to_export, IndicationDevice::Ded as i32)?;
        let (kind, program) = get_cmds_program(&tree)?;

        if (Countermeasure::Flare, ii + 1) != (kind, program) {
            log::error!(
                "Was not on correct page, expected {:?}, {}, got {:?}, {}",
                Countermeasure::Flare,
                ii + 1,
                kind,
                program
            );
            return None;
        }

        avionics.cmds.programs[ii as usize].flare = parse_cmds_program_page(&tree)?;

        ded_rocker_up(to_gamegui);
        wait_frame(to_gamegui);
    }

    Some(avionics)
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum IndicationDevice {
    Hud = 1,
    LeftMfd = 4,
    RightMfd = 5,
    Ded = 6,
    UhfRadioPreset = 10,
    UhfRadioFreq = 11,
    Ehsi = 13,
    CmdsQuantity = 16,
    Rwr = 17,
    Hmcs = 18,
}

fn get_avionics_indication(lua: &Lua, device: IndicationDevice) -> LuaResult<String> {
    list_indication(lua, device as i32)
}

#[trace(logging)]
fn get_avionics_value(
    to_export: &TaskSender<Lua>,
    device: IndicationDevice,
    path: &Vec<&str>,
) -> Option<String> {
    super::get_avionics_value(&to_export, device as i32, path)
}

#[trace(logging)]
fn get_hud_align_value(to_export: &TaskSender<Lua>) -> Option<String> {
    get_avionics_value(
        &to_export,
        IndicationDevice::Hud,
        &vec![
            "HUD_BlankRoot_PH_com",
            "HUD_Indication_bias",
            "HUD_Window7_origin",
            "HUD_AlignStatus_origin",
            "HUD_Window7_AlignmentStatus",
        ],
    )
}

#[trace(logging)]
fn is_on_cni(to_export: &TaskSender<Lua>) -> bool {
    get_avionics_value(to_export, IndicationDevice::Ded, &vec!["DED CNI TACAN PH"]).is_some()
}

#[trace(logging)]
fn is_on_list(to_export: &TaskSender<Lua>) -> bool {
    get_avionics_value(to_export, IndicationDevice::Ded, &vec!["LIST Label"]).is_some()
}

#[trace(logging)]
fn is_on_cmds_bingo(to_export: &TaskSender<Lua>) -> bool {
    get_avionics_value(to_export, IndicationDevice::Ded, &vec!["CMDS_BINGO_label"]).is_some()
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Countermeasure {
    Chaff,
    Flare,
}

#[trace(logging, disable(tree))]
fn get_cmds_program(tree: &slab_tree::Tree<IndicationNode>) -> Option<(Countermeasure, i8)> {
    lookup_tree(tree, &vec!["CMDS_Prog_label"])?;

    let program_val = parse_quantity(tree, &vec!["CMDS_Selected_Program"]);
    let chaff_val = lookup_tree(tree, &vec!["CMDS_CHAFF_label"]);
    let flare_val = lookup_tree(tree, &vec!["CMDS_FLARE_label"]);
    if chaff_val.is_some_and(|val| val.value == "CMDS CHAFF") {
        Some((Countermeasure::Chaff, program_val?))
    } else if flare_val.is_some_and(|val| val.value == "CMDS FLARE") {
        Some((Countermeasure::Flare, program_val?))
    } else {
        None
    }
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
                (Switch::InsKnob, 0.1),
            ];
            for (switch, state) in switch_states {
                let _ = set_switch_state(lua, switch, state);
            }
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
        self.get_elapsed_time(now) >= self.expire
    }

    pub fn get_elapsed_time(&self, now: f32) -> f32 {
        now - self.start_time
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
    WaitHudAlignMessage,
    WaitNoHudAlignMessage,
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
    startup_timer: Timer,
    avionics: AvionicsState,
}

const F16_STARTUP_TIME_MAX_SECONDS: f32 = 136.0;

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
            StartupState::WaitHudAlignMessage => self.wait_hud_align_message(event),
            StartupState::WaitNoHudAlignMessage => self.wait_no_hud_align_message(event),
            StartupState::Done => self.done(event),
        }
        if self.state != StartupState::ColdDark && self.state != StartupState::Done {
            self.gui.set_startup_progress(
                self.startup_timer.get_elapsed_time(sim_time) / F16_STARTUP_TIME_MAX_SECONDS,
            );
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
            startup_timer: Timer::default(),
            avionics: AvionicsState::default(),
        }
    }
    fn cold_dark_handler(&mut self, event: crate::app::FsmMessage) {
        match event {
            crate::app::FsmMessage::StartupAircraft => {
                self.startup_timer = Timer::new(0.0, self.sim_time);
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
            .send(|lua| get_avionics_indication(lua, IndicationDevice::Ded))
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

        let _ = set_three_pos(&self.to_gamegui, Switch::AntiSkid, ThreePosToggleState::Up);
        self.to_gamegui.send(|lua| {
            let _ =
                set_three_pos_springloaded(lua, Switch::AltimeterModeLever, ThreePosState::Stop);
        });
        self.gui.set_startup_text("Waiting for INS alignment");
        self.state = StartupState::WaitHudAlignMessage;
    }

    fn wait_hud_align_message(&mut self, event: crate::app::FsmMessage) {
        let val = get_hud_align_value(&self.to_export);
        let Some(txt) = val else {
            return;
        };
        if txt != "ALIGN" {
            return;
        }
        self.state = StartupState::WaitNoHudAlignMessage;
    }

    fn wait_no_hud_align_message(&mut self, event: crate::app::FsmMessage) {
        let val = get_hud_align_value(&self.to_export);
        let None = val else {
            return;
        };
        self.state = StartupState::Done;
        let _ = self
            .to_gamegui
            .send(|lua| set_switch_state(lua, Switch::InsKnob, 0.3))
            .wait();

        log::info!(
            "Finished startup in {} seconds",
            self.startup_timer.get_elapsed_time(self.sim_time)
        );
        self.gui.set_startup_progress(1.0);
        ded_return(&self.to_gamegui);
        if !is_on_cni(&self.to_export) {
            log::warn!("Not on CNI page after dobbering left!");
        }

        self.gui.set_startup_text("DONE");
    }

    fn done(&self, event: crate::app::FsmMessage) {}
}

fn bool_to_on_off(state: bool) -> &'static str {
    if state {
        "ON"
    } else {
        "OFF"
    }
}

fn add_quantity<T>(row: &mut egui_extras::TableRow, quantity: T)
where
    T: std::string::ToString,
{
    row.col(|ui| {
        ui.label(quantity.to_string());
    });
}

fn make_slot_row(row: &mut egui_extras::TableRow, cmds_program_slot: &CmdsProgramSlot) {
    add_quantity(row, cmds_program_slot.burst_quantity);
    add_quantity(row, cmds_program_slot.burst_interval);
    add_quantity(row, cmds_program_slot.sequence_quantity);
    add_quantity(row, cmds_program_slot.sequence_interval);
}

#[derive(Default, Debug, Clone)]
struct EditTracker {
    pub edited: bool,
}

impl EditTracker {
    fn update(&mut self, updated: bool) {
        self.edited = self.edited || updated;
    }
    fn add_text_parser<T, F>(
        &mut self,
        ui: &mut egui::Ui,
        txt: &mut String,
        val: &T,
        updated_val: &mut T,
        parser: F,
    ) where
        F: FnOnce(&String) -> T,
        T: FromStr + Display,
    {
        ui.label(format!("{}", val));
        let edited = ui.text_edit_singleline(txt).changed();
        self.update(edited);
        *updated_val = parser(txt);
    }

    fn add_bool(&mut self, ui: &mut egui::Ui, val: &bool, updated_val: &mut bool) {
        ui.label(bool_to_on_off(*val));
        self.update(ui.checkbox(updated_val, "").changed());
    }
}

#[derive(Default, Debug, Clone)]
pub struct Gui {
    avionics: Arc<Mutex<AvionicsState>>,
    avionics_updated: AvionicsState,
    edit_tracker: EditTracker,
    flare_bingo_quantity_raw: String,
    chaff_bingo_quantity_raw: String,
}

impl Gui {
    fn update_text_fields(&mut self) {
        self.flare_bingo_quantity_raw = self.avionics_updated.cmds.bingo.flare.to_string();
        self.chaff_bingo_quantity_raw = self.avionics_updated.cmds.bingo.chaff.to_string();
    }

    pub fn make_widget(
        &mut self,
        ui: &mut egui::Ui,
        to_app: &TaskSender<(TaskSender<Lua>, TaskSender<Lua>)>,
    ) {
        let countermeasures_section = move |ui: &mut egui::Ui| {
            let s = self.avionics.clone();
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size;

            ui.horizontal(|ui| {
                if ui.button("Read").clicked() {
                    let _ = to_app
                        .send(move |(to_gamegui, to_export)| {
                            let result = read_cmds(&to_gamegui, &to_export);
                            *s.clone().lock().unwrap() = result.unwrap();
                        })
                        .wait();
                    self.avionics_updated = self.avionics.lock().unwrap().clone();
                    self.update_text_fields();
                }

                let apply_button =
                    ui.add_enabled(self.edit_tracker.edited, egui::Button::new("Apply"));
                if apply_button.clicked() {
                    self.edit_tracker.edited = false;
                    *self.avionics.lock().unwrap() = self.avionics_updated.clone();
                }
            });

            let strong_heading =
                |ui: &mut egui::Ui, txt| ui.heading(egui::RichText::new(txt).strong());

            let cmds = &self.avionics.lock().unwrap().cmds;
            let cmds_updated = &mut self.avionics_updated.cmds;

            let parse_bingo_quantity = |raw: &String| {
                let val: i8 = raw.parse().unwrap_or(0);
                val.clamp(0, 99)
            };

            ui.separator();
            let grid = egui::Grid::new("cmds_bingo_grid");
            let grid = grid.num_columns(3).striped(true).show(ui, |ui| {
                ui.strong("Bingo quantities");
                ui.end_row();
                ui.label("Flare bingo quantity");
                self.edit_tracker.add_text_parser(
                    ui,
                    &mut self.flare_bingo_quantity_raw,
                    &cmds.bingo.flare,
                    &mut cmds_updated.bingo.flare,
                    parse_bingo_quantity,
                );
                ui.end_row();

                ui.label("Chaff bingo quantity");
                self.edit_tracker.add_text_parser(
                    ui,
                    &mut self.chaff_bingo_quantity_raw,
                    &cmds.bingo.chaff,
                    &mut cmds_updated.bingo.chaff,
                    parse_bingo_quantity,
                );
                ui.end_row();

                ui.strong("Audible warnings");
                ui.end_row();

                ui.label("Feedback");
                self.edit_tracker.add_bool(
                    ui,
                    &cmds.bingo.feedback,
                    &mut cmds_updated.bingo.feedback,
                );
                ui.end_row();

                ui.label("Request countermeasures");
                self.edit_tracker
                    .add_bool(ui, &cmds.bingo.reqctr, &mut cmds_updated.bingo.reqctr);
                ui.end_row();

                ui.label("Bingo");
                self.edit_tracker
                    .add_bool(ui, &cmds.bingo.bingo, &mut cmds_updated.bingo.bingo);

                ui.end_row();
            });

            ui.separator();
            strong_heading(ui, "Programs");
            use egui_extras::{Column, TableBuilder};

            let table = TableBuilder::new(ui)
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto());

            table
                .header(text_height * 1.1, |mut header| {
                    header.col(|ui| {
                        ui.strong("");
                    });
                    header.col(|ui| {
                        ui.strong("Burst quantity");
                    });
                    header.col(|ui| {
                        ui.strong("Burst interval");
                    });
                    header.col(|ui| {
                        ui.strong("Sequence quantity");
                    });
                    header.col(|ui| {
                        ui.strong("Sequence interval");
                    });
                })
                .body(|mut body| {
                    for idx in 0..6 {
                        body.row(text_height, |mut row| {
                            row.col(|ui| {
                                ui.strong(format!("Program {} chaff", idx + 1));
                            });
                            make_slot_row(&mut row, &cmds.programs[idx].chaff);
                        });
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.strong(format!("Program {} flare", idx + 1));
                            });
                            make_slot_row(&mut row, &cmds.programs[idx].flare);
                        });
                    }
                })
        };
        egui::CollapsingHeader::new("Countermeasures")
            .default_open(false)
            .show(ui, countermeasures_section);
    }
}
