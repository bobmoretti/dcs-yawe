use serde::{Deserialize, Serialize};
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub write_dir: String,
    pub lua_path: String,
    pub dll_path: String,
    pub log_level: String,
}

impl<'lua> mlua::FromLua<'lua> for Config {
    fn from_lua(lua_value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        use mlua::LuaSerdeExt;
        let config: Config = lua.from_value(lua_value)?;
        Ok(config)
    }
}
