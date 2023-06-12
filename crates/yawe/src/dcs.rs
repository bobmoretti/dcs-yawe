use mlua::prelude::{LuaFunction, LuaTable};
use mlua::Lua;

pub fn get_ownship_type(lua: &Lua) -> String {
    let export: LuaTable = lua.globals().get("Export").unwrap();
    let get_self_data: LuaFunction = export.get("LoGetSelfData").unwrap();
    let self_data: LuaTable = get_self_data.call(()).unwrap();
    self_data.get("Name").unwrap()
}
