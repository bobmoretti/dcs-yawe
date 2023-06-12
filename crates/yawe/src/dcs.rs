use mlua::prelude::{LuaFunction, LuaTable};
use mlua::Lua;

#[derive(PartialEq, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum AircraftType {
    F_16C_50,
    UNKNOWN,
}

fn str_to_ship_enum(name: &str) -> AircraftType {
    match name {
        "F-16C_50" => AircraftType::F_16C_50,
        _ => AircraftType::UNKNOWN,
    }
}

pub fn get_ownship_type(lua: &Lua) -> AircraftType {
    let export: LuaTable = lua.globals().get("Export").unwrap();
    let get_self_data: LuaFunction = export.get("LoGetSelfData").unwrap();
    let self_data: LuaTable = get_self_data.call(()).unwrap();
    let s: String = self_data.get("Name").unwrap();
    str_to_ship_enum(s.as_str())
}
