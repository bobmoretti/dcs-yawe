use libloading;
use mlua::prelude::{LuaResult, LuaTable};
use mlua::Lua;
use std::path::Path;
use ouroboros::self_referencing;

#[self_referencing]
struct LibState {
    lib: libloading::Library,
    #[borrows(lib)]
    #[covariant]
    start: libloading::Symbol<'this, extern "C" fn(config::Config) -> u32>,
    #[borrows(lib)]
    #[covariant]
    stop: libloading::Symbol<'this, extern "C" fn() -> u32>,
    // on_frame: libloading::Symbol<'static, unsafe extern "C" fn() -> u32>,
}

static mut LIB_STATE: Option<LibState> = None;

fn load_export<'a, T>(lib: &'a libloading::Library, name: &[u8]) -> libloading::Symbol<'a, T> {
    unsafe { lib.get::<T>(name).unwrap() }
}

#[no_mangle]
pub fn start(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    let dll_path = Path::new(config.dll_path.as_str()).join("yawe.dll");
    unsafe {
        LIB_STATE = Some(LibStateBuilder {
            lib: libloading::Library::new(dll_path).unwrap(),
            start_builder: |l: &libloading::Library| load_export(l, b"start"),
            stop_builder: |l: &libloading::Library| load_export(l, b"stop"),
        }.build());
        let res = LIB_STATE.as_ref().unwrap().borrow_start()(config);
        Ok(res as i32)
    }
}

#[no_mangle]
pub fn stop(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    Ok(unsafe { LIB_STATE.as_ref().unwrap() }.borrow_stop()() as i32)
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
