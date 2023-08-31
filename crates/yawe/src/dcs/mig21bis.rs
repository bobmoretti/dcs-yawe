use crate::dcs;
use mlua::prelude::LuaResult;
use mlua::Lua;
use strum::IntoStaticStr;

#[derive(Debug, Clone, Copy, IntoStaticStr)]
#[allow(dead_code)]
pub enum Switch {
    FuelPump1,
    FuelPump3,
    FuelPumpDrain,
    BatteryOn,
    BatteryHeat,
    AcGenerator,
    DcGenerator,
    SprdPower,
    SprdDropPower,
    Po750Inverter1,
    Po750Inverter2,
    ApuPower,
    FireExtinguisherPower,
    ThrottleStopLock,
    CanopyOpen,
    CanopyClose,
    CanopyLock,
    CanopySeal,
    NumSwitches,
}

pub struct SwitchInfo {
    pub _switch: Switch,
    pub device_id: i32,
    pub command: i32,
    pub argument: i32,
}

pub static SWITCH_INFO_MAP: [SwitchInfo; Switch::NumSwitches as usize] = [
    SwitchInfo::new(Switch::FuelPump1, 4, 3011, 160),
    SwitchInfo::new(Switch::FuelPump3, 4, 3010, 159),
    SwitchInfo::new(Switch::FuelPumpDrain, 4, 3012, 161),
    SwitchInfo::new(Switch::BatteryOn, 1, 3001, 165),
    SwitchInfo::new(Switch::BatteryHeat, 1, 3002, 155),
    SwitchInfo::new(Switch::AcGenerator, 2, 3004, 169),
    SwitchInfo::new(Switch::DcGenerator, 1, 3003, 166),
    SwitchInfo::new(Switch::SprdPower, 48, 3106, 167),
    SwitchInfo::new(Switch::SprdDropPower, 48, 3107, 168),
    SwitchInfo::new(Switch::Po750Inverter1, 2, 3005, 153),
    SwitchInfo::new(Switch::Po750Inverter2, 2, 3006, 154),
    SwitchInfo::new(Switch::ApuPower, 3, 3014, 302),
    SwitchInfo::new(Switch::FireExtinguisherPower, 53, 3025, 303),
    SwitchInfo::new(Switch::ThrottleStopLock, 3, 3238, 616),
    SwitchInfo::new(Switch::CanopyOpen, 43, 3152, 375),
    SwitchInfo::new(Switch::CanopyClose, 43, 3194, 385),
    SwitchInfo::new(Switch::CanopyLock, 43, 3151, 329),
    SwitchInfo::new(Switch::CanopySeal, 43, 3150, 328),
];

impl SwitchInfo {
    pub const fn new(_switch: Switch, device_id: i32, command: i32, argument: i32) -> Self {
        Self {
            _switch,
            device_id,
            command,
            argument,
        }
    }
}

fn get_switch_info(s: Switch) -> &'static SwitchInfo {
    &SWITCH_INFO_MAP[s as usize]
}

fn toggle_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    let info = get_switch_info(s);
    dcs::perform_click(lua, info.device_id, info.command, 1.0)
}

pub fn get_switch_state(lua: &Lua, s: Switch) -> LuaResult<f64> {
    let info = get_switch_info(s);
    dcs::get_switch_state(lua, 0, info.argument)
}

pub fn is_switch_set(lua: &Lua, s: Switch) -> LuaResult<bool> {
    Ok(get_switch_state(lua, s)? > 0.5)
}

pub fn set_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    if !is_switch_set(lua, s)? {
        toggle_switch(lua, s)
    } else {
        Ok(())
    }
}

pub fn unset_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    if is_switch_set(lua, s)? {
        toggle_switch(lua, s)
    } else {
        Ok(())
    }
}
