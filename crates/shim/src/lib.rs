use mlua::prelude::{LuaResult, LuaTable};
use mlua::Lua;
use std::path::Path;
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{FARPROC, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

// Implementation based on https://github.com/microsoft/windows-rs/issues/973#issuecomment-1363481060
struct PCWSTRWrapper {
    text: PCWSTR,
    // this is here to allow it to get dropped at the same time as the PCWSTR
    #[allow(unused)]
    _container: Vec<u16>,
}

impl std::ops::Deref for PCWSTRWrapper {
    type Target = PCWSTR;

    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

trait ToPCWSTRWrapper {
    fn to_pcwstr(&self) -> PCWSTRWrapper;
}

impl ToPCWSTRWrapper for &str {
    fn to_pcwstr(&self) -> PCWSTRWrapper {
        // do not drop when scope ends, by moving it into struct
        let mut text = self.encode_utf16().collect::<Vec<_>>();
        text.push(0);

        PCWSTRWrapper {
            text: PCWSTR::from_raw(text.as_ptr()),
            _container: text,
        }
    }
}

impl ToPCWSTRWrapper for Path {
    fn to_pcwstr(&self) -> PCWSTRWrapper {
        self.to_str().unwrap().to_pcwstr()
    }
}

struct LibState {
    lib: HMODULE,
    start: ProcWrapper<extern "C" fn(config::Config) -> u32>,
    stop: ProcWrapper<extern "C" fn() -> u32>,
    on_frame: ProcWrapper<extern "C" fn() -> u32>,
}

static mut LIB_STATE: Option<LibState> = None;

struct ProcWrapper<T> {
    proc: FARPROC,
    pd: std::marker::PhantomData<T>,
}

impl<T> ::std::ops::Deref for ProcWrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*((&self.proc) as *const FARPROC as *const T) }
    }
}

fn load_export<T>(lib: &HMODULE, name: &[u8]) -> ProcWrapper<T> {
    unsafe {
        ProcWrapper::<T> {
            proc: GetProcAddress::<HMODULE, PCSTR>(
                *lib,
                windows::core::PCSTR(name.as_ptr().cast()),
            ),
            pd: std::marker::PhantomData,
        }
    }
}

#[no_mangle]
pub fn start(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    let dll_path = Path::new(config.dll_path.as_str()).join("yawe.dll");
    unsafe {
        let lib = LoadLibraryW(*dll_path.to_pcwstr()).unwrap();

        LIB_STATE = Some(LibState {
            lib: lib,
            start: load_export(&lib, b"start\0"),
            stop: load_export(&lib, b"stop\0"),
            on_frame: load_export(&lib, b"on_frame\0")
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
