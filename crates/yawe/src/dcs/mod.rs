pub mod f16c50;
pub mod mig21bis;

use crate::app::FsmMessage;
use crate::Error;
use mlua::prelude::{LuaFunction, LuaResult, LuaTable};
use mlua::Lua;
use offload::TaskSender;
use std::str::FromStr;

pub struct SwitchInfo<SwitchT> {
    pub switch: SwitchT,
    pub device_id: i32,
    pub command: i32,
    pub argument: i32,
}

impl<SwitchT> SwitchInfo<SwitchT> {
    pub const fn new(switch: SwitchT, device_id: i32, command: i32, argument: i32) -> Self {
        Self {
            switch: switch,
            device_id,
            command,
            argument,
        }
    }

    pub const fn new_float(switch: SwitchT, argument: i32) -> Self {
        Self {
            switch,
            device_id: 0,
            command: 0,
            argument,
        }
    }
}

pub trait AircraftFsm {
    fn run_fsm(&mut self, msg: FsmMessage, sim_time: f32);
}

#[derive(PartialEq, Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum AircraftId {
    A_10C,
    A_10C_2,
    AH_64D_BLK_II,
    AJS37,
    AV8BNA,
    F_14B,
    F_15ESE,
    F_15ESE_WSO,
    F_16C_50,
    FA_18C_hornet,
    M_2000C,
    Mi_24P,
    Mi_8MT,
    Mi_8MT_Copilot,
    Mi_8MT_FO,
    MiG_21Bis,
    SA342L,
    Su_25,
    Su_25T,
    UH_1H,
    Unknown(String),
}

fn str_to_ship_enum(name: &str) -> AircraftId {
    match name {
        "A-10C" => AircraftId::A_10C,
        "A-10C_2" => AircraftId::A_10C_2,
        "AH-64D_BLK_II" => AircraftId::AH_64D_BLK_II,
        "AV8BNA" => AircraftId::AV8BNA,
        "AJS37" => AircraftId::AJS37,
        "F-14B" => AircraftId::F_14B,
        "F-15ESE" => AircraftId::F_15ESE,
        "F-15ESE WSO" => AircraftId::F_15ESE_WSO,
        "F-16C_50" => AircraftId::F_16C_50,
        "FA-18C_hornet" => AircraftId::FA_18C_hornet,
        "M-2000C" => AircraftId::M_2000C,
        "Mi-24P" => AircraftId::Mi_24P,
        "Mi-8MT" => AircraftId::Mi_8MT,
        "Mi-8MT Copilot" => AircraftId::Mi_8MT_Copilot,
        "Mi-8MT FO" => AircraftId::Mi_8MT_FO,
        "MiG-21Bis" => AircraftId::MiG_21Bis,
        "SA342L" => AircraftId::SA342L,
        "Su-25" => AircraftId::Su_25,
        "Su-25T" => AircraftId::Su_25T,
        "UH-1H" => AircraftId::UH_1H,
        _ => AircraftId::Unknown(String::from_str(name).unwrap()),
    }
}

pub struct EmptyFsm {}
impl AircraftFsm for EmptyFsm {
    fn run_fsm(&mut self, _: FsmMessage, _: f32) {}
}

impl EmptyFsm {
    pub fn new(_: TaskSender<Lua>, _: TaskSender<Lua>, _: crate::gui::TxHandle) -> Self {
        Self {}
    }
}

pub fn get_aircraft(
    aircraft: AircraftId,
    to_gamegui: TaskSender<Lua>,
    to_export: TaskSender<Lua>,
    gui: crate::gui::TxHandle,
) -> Box<dyn AircraftFsm> {
    match aircraft {
        AircraftId::MiG_21Bis => Box::new(mig21bis::Fsm::new(to_gamegui, to_export, gui)),
        AircraftId::F_16C_50 => Box::new(f16c50::Fsm::new(to_gamegui, to_export, gui)),
        _ => Box::new(EmptyFsm::new(to_gamegui, to_export, gui)),
    }
}

pub fn get_ownship_type(lua: &Lua) -> LuaResult<AircraftId> {
    let export: LuaTable = lua.globals().get("Export")?;
    let get_self_data: LuaFunction = export.get("LoGetSelfData")?;
    let self_data: LuaTable = get_self_data.call(())?;
    let s: String = self_data.get("Name")?;
    Ok(str_to_ship_enum(s.as_str()))
}

fn get_cockpit_device(lua: &Lua, device_id: i32) -> LuaResult<LuaTable> {
    let export: LuaTable = lua.globals().get("Export")?;
    let get_device: LuaResult<LuaFunction> = export.get("GetDevice");
    if get_device.is_err() {
        log::info!("Failed to get device {device_id}");
    }
    get_device.unwrap().call(device_id)
}

pub fn perform_click(lua: &Lua, device_id: i32, command: i32, value: f32) -> LuaResult<()> {
    let device: LuaTable = get_cockpit_device(lua, device_id)?;
    let perform_click: LuaFunction = device.get("performClickableAction")?;
    perform_click.call((device, command, value))
}

pub fn get_switch_state(lua: &Lua, device_id: i32, command: i32) -> LuaResult<f32> {
    let device: LuaTable = get_cockpit_device(lua, device_id)?;
    let get_value: LuaResult<LuaFunction> = device.get("get_argument_value");
    if let Err(e) = get_value {
        log::warn!(
            "Could not find function get_argument_value, result is {:?}",
            e
        );
        return Err(e);
    }
    get_value.unwrap().call((device, command))
}

pub fn set_command(lua: &Lua, command: i32, value: f32) -> LuaResult<f32> {
    let export: LuaTable = lua.globals().get("Export")?;
    let set_command: LuaResult<LuaFunction> = export.get("LoGetCommand");
    if let Err(e) = set_command {
        log::warn!("Could not find function LoGetCommand, result is {:?}", e);
        return Err(e);
    }
    set_command.unwrap().call((command, value))
}

pub fn list_cockpit_params(lua: &Lua) -> LuaResult<String> {
    let list_cockpit_params: LuaResult<LuaFunction> = lua.globals().get("list_cockpit_params");
    if let Err(e) = list_cockpit_params {
        log::warn!(
            "Could not find global function list_cockpit_params, result is {:?}",
            e
        );
        return Err(e);
    }
    list_cockpit_params.unwrap().call(())
}

pub fn get_cockpit_param(lua: &Lua, param_name: &str) -> std::result::Result<f32, Error> {
    let params = list_cockpit_params(lua).map_err(|e| Error::LuaError(e))?;
    let pattern = [param_name, ":"].join("");

    for line in params.split("\n") {
        if line.trim().starts_with(&pattern) {
            let mut s = line.split(":");
            s.next();
            let val = s.next();
            if let None = val {
                return Err(Error::ParseError(line.into()));
            }
            return Ok(val
                .unwrap()
                .parse()
                .map_err(|_| Error::ParseError(line.into()))?);
        }
    }
    return Err(Error::IndexError);
}

pub fn is_paused(lua: &Lua) -> LuaResult<bool> {
    let dcs: LuaTable = lua.globals().get("DCS")?;
    let get_pause: LuaFunction = dcs.get("getPause")?;
    get_pause.call(())
}

pub fn get_sim_time(lua: &Lua) -> LuaResult<f32> {
    let dcs: LuaTable = lua.globals().get("DCS")?;
    let get_model_time: LuaFunction = dcs.get("getModelTime")?;
    get_model_time.call(())
}

pub enum LockonCommand {
    LeftEngineStart = 311,
    RightEngineStart = 312,
    LeftEngineStop = 313,
    RightEngineStop = 314,
}

pub fn set_lockon_command(lua: &Lua, command: LockonCommand) -> LuaResult<()> {
    let export: LuaTable = lua.globals().get("Export")?;
    let send_command: LuaFunction = export.get("LoSetCommand")?;
    send_command.call(command as i32)
}
