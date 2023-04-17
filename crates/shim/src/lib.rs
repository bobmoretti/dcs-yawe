use libloading;
use mlua::prelude::{LuaResult, LuaTable};
use mlua::Lua;
use std::path::Path;

struct LibState {
    lib: libloading::Library,
    start: libloading::Symbol<'static, extern "C" fn(config::Config) -> u32>,
    stop: libloading::Symbol<'static, extern "C" fn() -> u32>,
    // on_frame: libloading::Symbol<'static, unsafe extern "C" fn() -> u32>,
}

static mut LIB_STATE: Option<LibState> = None;

fn load_export<T>(lib: &libloading::Library, name: &[u8]) -> libloading::Symbol<'static, T> {
    unsafe { std::mem::transmute(lib.get::<T>(name).unwrap()) }
}

#[no_mangle]
pub fn start(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    let dll_path = Path::new(config.dll_path.as_str()).join("yawe.dll");
    unsafe {
        let lib = libloading::Library::new(dll_path).unwrap();
        let start = load_export(&lib, b"start");
        let stop = load_export(&lib, b"stop");

        LIB_STATE = Some(LibState {
            lib: lib,
            start: start,
            stop: stop,
        });
    }

    Ok(0)
}

#[no_mangle]
pub fn stop(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    Ok(0)
}

#[mlua::lua_module]
pub fn yawe(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("start", lua.create_function(start)?)?;
    // exports.set("on_frame_begin", lua.create_function(on_frame_begin)?)?;
    // exports.set("on_frame_end", lua.create_function(on_frame_end)?)?;
    exports.set("stop", lua.create_function(stop)?)?;
    Ok(exports)
}
