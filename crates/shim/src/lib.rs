use mlua::prelude::{LuaResult, LuaTable};
use mlua::Lua;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{FARPROC, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

struct LibState {
    lib: HMODULE,
    start: Symbol<extern "C" fn(config::Config) -> u32>,
    stop: Symbol<extern "C" fn() -> u32>,
    // on_frame: libloading::Symbol<'static, unsafe extern "C" fn() -> u32>,
}

static mut LIB_STATE: Option<LibState> = None;

fn load_library(p: &Path) -> windows::core::Result<HMODULE> {
    use windows::core::PCWSTR;
    let wide_path: Vec<u16> = p.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe { LoadLibraryW(PCWSTR(wide_path.as_ptr())) }
}

struct Symbol<T> {
    pointer: FARPROC,
    _pd: std::marker::PhantomData<T>,
}

impl<T> std::ops::Deref for Symbol<T> {
    type Target = T;
    fn deref(&self) -> &T {
        assert!(self.pointer.is_some());
        // Shield your eyes.
        unsafe { &*((&self.pointer) as *const FARPROC as *const T) }
    }
}

fn load_export<T>(lib: HMODULE, sym: &[u8]) -> Symbol<T> {
    use windows::core::PCSTR;

    // Compile-time check?
    assert_eq!(std::mem::size_of::<T>(), std::mem::size_of::<FARPROC>());

    let terminated = std::ffi::CString::new(sym).expect("Null bytes in sym");
    let pointer = unsafe { GetProcAddress(lib, PCSTR(terminated.as_ptr() as *const u8)) };
    Symbol {
        pointer,
        _pd: std::marker::PhantomData,
    }
}

#[no_mangle]
pub fn start(lua: &Lua, config: config::Config) -> LuaResult<i32> {
    let dll_path = Path::new(config.dll_path.as_str()).join("yawe.dll");
    unsafe {
        let lib = load_library(&dll_path).unwrap();
        let start = load_export(lib, b"start");
        let stop = load_export(lib, b"stop");

        LIB_STATE = Some(LibState { lib, start, stop });
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
