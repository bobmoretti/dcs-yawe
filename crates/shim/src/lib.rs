use log::LevelFilter;
use mlua::prelude::{LuaResult, LuaTable};
use mlua::Lua;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{BOOL, FARPROC, HMODULE};
use windows::Win32::System::LibraryLoader::{FreeLibrary, GetProcAddress, LoadLibraryW};

struct LibState {
    lib: HMODULE,
    start: ProcWrapper<extern "C" fn(&Lua, config::Config) -> i32>,
    stop: ProcWrapper<extern "C" fn(&Lua) -> i32>,
    on_frame: ProcWrapper<extern "C" fn(&Lua) -> i32>,
}

static mut LIB_STATE: Option<LibState> = None;

struct ProcWrapper<T> {
    proc: FARPROC,
    _pd: std::marker::PhantomData<T>,
}

impl<T> std::ops::Deref for ProcWrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        assert!(self.proc.is_some());
        unsafe { &*((&self.proc) as *const FARPROC as *const T) }
    }
}

fn load_export<T>(lib: HMODULE, sym: &[u8]) -> ProcWrapper<T> {
    use windows::core::PCSTR;

    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<FARPROC>());

    let terminated = std::ffi::CString::new(sym).expect("Null bytes in sym");
    let pointer = unsafe { GetProcAddress(lib, PCSTR(terminated.as_ptr() as *const u8)) };
    ProcWrapper {
        proc: pointer,
        _pd: std::marker::PhantomData,
    }
}

fn load_library(p: &Path) -> windows::core::Result<HMODULE> {
    use windows::core::PCWSTR;
    let wide_path: Vec<u16> = p.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe { LoadLibraryW(PCWSTR(wide_path.as_ptr())) }
}

fn setup_logging(write_dir: &str) {
    let logdir = Path::new(write_dir).join("Logs").join("Yawe");
    std::fs::create_dir_all(&logdir).expect("Unable to create log file");
    let p = logdir.join("shim.log");
    simple_logging::log_to_file(p.as_os_str(), LevelFilter::Info)
        .expect("Unable to create log file");
}

fn close_library() -> BOOL {
    let free_result = unsafe { FreeLibrary(LIB_STATE.as_ref().unwrap().lib) };
    log::info!("Freeing library result: {:?}", free_result);
    unsafe { LIB_STATE = None };
    free_result
}

#[no_mangle]
pub fn start(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    setup_logging(&config.write_dir);
    log::info!("Log file created.");
    let dll_path = Path::new(config.dll_path.as_str()).join("yawe.dll");
    let lib = load_library(&dll_path).unwrap();
    let ls = Some(LibState {
        lib: lib,
        start: load_export(lib, b"start"),
        stop: load_export(lib, b"stop"),
        on_frame: load_export(lib, b"on_frame"),
    });
    unsafe { LIB_STATE = ls };

    let start = unsafe { &LIB_STATE.as_ref().unwrap().start };
    let result = start(&lua, config);

    Ok(result)
}

#[no_mangle]
pub fn stop(lua: &Lua, _: ()) -> LuaResult<i32> {
    if !unsafe { LIB_STATE.is_some() } {
        return Ok(-1);
    }
    let stop = unsafe { &LIB_STATE.as_ref().unwrap().stop };
    let stop_result = stop(&lua);
    log::info!("Stopping main library returned {:?}", stop_result);
    let free_result = close_library();
    Ok(free_result.as_bool().into())
}

#[no_mangle]
pub fn on_frame(lua: &Lua, _: ()) -> LuaResult<i32> {
    let maybe_lib_state = unsafe { &LIB_STATE.as_ref() };
    if let None = &maybe_lib_state {
        return Ok(-1);
    }
    let on_frame = &maybe_lib_state.unwrap().on_frame;
    let result = on_frame(&lua);
    if result < 0 {
        log::info!("Development: user asked to close library\n");
        close_library();
    }
    Ok(result)
}

#[mlua::lua_module]
pub fn yawe_shim(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("start", lua.create_function(start)?)?;
    exports.set("on_frame", lua.create_function(on_frame)?)?;
    exports.set("stop", lua.create_function(stop)?)?;
    Ok(exports)
}
