use mlua::prelude::LuaResult;
use mlua::Lua;
use std::thread::JoinHandle;
mod app;
mod gui;

struct LibState {
    main_thread: JoinHandle<()>,
}

static mut LIB_STATE: Option<LibState> = None;

#[no_mangle]
pub fn start(_lua: &Lua, config: config::Config) -> LuaResult<i32> {
    let main_thread = std::thread::spawn(|| app::entry());
    unsafe {
        LIB_STATE = Some(LibState { main_thread });
    }
    Ok(0)
}

#[no_mangle]
pub fn stop(_lua: &Lua) -> LuaResult<i32> {
    unsafe {
        LIB_STATE = None;
    }
    Ok(0)
}

#[no_mangle]
pub fn on_frame(_lua: &Lua) -> LuaResult<i32> {
    Ok(0)
}
