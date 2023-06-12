use mlua::prelude::{LuaFunction, LuaTable};
use mlua::Lua;
use std::string::String;
mod app;
mod dcs;
mod gui;
mod logging;

struct LibState {
    main_app: app::App,
}

static mut LIB_STATE: Option<LibState> = None;

fn get_lib_state() -> &'static mut LibState {
    if let None = unsafe { LIB_STATE.as_mut() } {
        panic!("Library not initialized!");
    }
    unsafe { LIB_STATE.as_mut().unwrap() }
}

fn get_writedir(lua: &Lua) -> String {
    let lfs: LuaTable = lua.globals().get("lfs").unwrap();
    let get_writedir: LuaFunction = lfs.get("writedir").unwrap();
    get_writedir.call::<_, String>(()).unwrap()
}

#[no_mangle]
pub fn start(lua: &Lua, mut config: config::Config) -> i32 {
    config.write_dir = get_writedir(lua);
    logging::init(&config);
    unsafe {
        LIB_STATE = Some(LibState {
            main_app: app::App::new(),
        });
    }
    0
}

#[no_mangle]
pub fn stop(_lua: &Lua) -> i32 {
    get_lib_state().main_app.stop();
    unsafe {
        LIB_STATE = None;
    }
    0
}

#[no_mangle]
pub fn on_frame(lua: &Lua) -> i32 {
    get_lib_state().main_app.on_frame(&lua);
    0
}
