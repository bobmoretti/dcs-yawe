use mlua::prelude::{LuaFunction, LuaTable};
use mlua::Lua;
use std::thread::JoinHandle;
mod app;
mod gui;
mod logging;
use std::string::String;

struct LibState {
    main_thread: JoinHandle<()>,
}

static mut LIB_STATE: Option<LibState> = None;

fn get_writedir(lua: &Lua) -> String {
    let lfs: LuaTable = lua.globals().get("lfs").unwrap();
    let get_writedir: LuaFunction = lfs.get("writedir").unwrap();
    get_writedir.call::<_, String>(()).unwrap()
}

#[no_mangle]
pub fn start(lua: &Lua, mut config: config::Config) -> i32 {
    config.write_dir = get_writedir(lua);
    logging::init(&config);
    let main_thread = std::thread::spawn(|| app::entry());
    unsafe {
        LIB_STATE = Some(LibState { main_thread });
    }
    0
}

#[no_mangle]
pub fn stop(_lua: &Lua) -> i32 {
    unsafe {
        LIB_STATE = None;
    }
    0
}

#[no_mangle]
pub fn on_frame(_lua: &Lua) -> i32 {
    0
}
