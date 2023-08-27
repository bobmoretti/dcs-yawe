use std::str::FromStr;

use mlua::prelude::{LuaFunction, LuaTable};
use mlua::Lua;

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

pub fn get_ownship_type(lua: &Lua) -> AircraftId {
    let export: LuaTable = lua.globals().get("Export").unwrap();
    let get_self_data: LuaFunction = export.get("LoGetSelfData").unwrap();
    let self_data: LuaTable = get_self_data.call(()).unwrap();
    let s: String = self_data.get("Name").unwrap();
    str_to_ship_enum(s.as_str())
}
