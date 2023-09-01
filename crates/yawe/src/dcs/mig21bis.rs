use crate::dcs;
use mlua::prelude::LuaResult;
use mlua::Lua;
use offload::TaskSender;
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

#[derive(Debug, Copy, Clone)]
pub enum SwitchType {
    Toggle,
    Momentary,
}

pub struct SwitchInfo {
    pub _switch: Switch,
    pub device_id: i32,
    pub command: i32,
    pub argument: i32,
    pub switch_type: SwitchType,
}

type St = SwitchType;

pub static SWITCH_INFO_MAP: [SwitchInfo; Switch::NumSwitches as usize] = [
    SwitchInfo::new(Switch::FuelPump1, 4, 3011, 160, St::Toggle),
    SwitchInfo::new(Switch::FuelPump3, 4, 3010, 159, St::Toggle),
    SwitchInfo::new(Switch::FuelPumpDrain, 4, 3012, 161, St::Toggle),
    SwitchInfo::new(Switch::BatteryOn, 1, 3001, 165, St::Toggle),
    SwitchInfo::new(Switch::BatteryHeat, 1, 3002, 155, St::Toggle),
    SwitchInfo::new(Switch::AcGenerator, 2, 3004, 169, St::Toggle),
    SwitchInfo::new(Switch::DcGenerator, 1, 3003, 166, St::Toggle),
    SwitchInfo::new(Switch::SprdPower, 48, 3106, 167, St::Toggle),
    SwitchInfo::new(Switch::SprdDropPower, 48, 3107, 168, St::Toggle),
    SwitchInfo::new(Switch::Po750Inverter1, 2, 3005, 153, St::Toggle),
    SwitchInfo::new(Switch::Po750Inverter2, 2, 3006, 154, St::Toggle),
    SwitchInfo::new(Switch::ApuPower, 3, 3014, 302, St::Toggle),
    SwitchInfo::new(Switch::FireExtinguisherPower, 53, 3025, 303, St::Toggle),
    SwitchInfo::new(Switch::ThrottleStopLock, 3, 3238, 616, St::Toggle),
    SwitchInfo::new(Switch::CanopyOpen, 43, 3152, 375, St::Momentary),
    SwitchInfo::new(Switch::CanopyClose, 43, 3194, 385, St::Momentary),
    SwitchInfo::new(Switch::CanopyLock, 43, 3151, 329, St::Toggle),
    SwitchInfo::new(Switch::CanopySeal, 43, 3150, 328, St::Toggle),
];

enum StartupState {
    ColdDark,
    WaitCanopyClosed,
    WaitEngineStarted,
    Done,
}

impl SwitchInfo {
    pub const fn new(
        _switch: Switch,
        device_id: i32,
        command: i32,
        argument: i32,
        switch_type: SwitchType,
    ) -> Self {
        Self {
            _switch,
            device_id,
            command,
            argument,
            switch_type,
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

fn start_jet(lua: &Lua) {}

pub struct Fsm {
    state: StartupState,
    to_dcs_gamegui: TaskSender<Lua>,
    to_dcs_export: TaskSender<Lua>,
}

impl Fsm {
    pub fn new(to_dcs_gamegui: TaskSender<Lua>, to_dcs_export: TaskSender<Lua>) -> Self {
        Self {
            state: StartupState::ColdDark,
            to_dcs_gamegui,
            to_dcs_export,
        }
    }
    pub fn run(&mut self, event: crate::app::AppMessage) {
        match self.state {
            StartupState::ColdDark => self.cold_dark_handler(event),
            StartupState::WaitCanopyClosed => self.wait_canopy_closed(event),
            StartupState::WaitEngineStarted => self.wait_engine_started(event),
            StartupState::Done => self.done(event),
        }
    }
    fn cold_dark_handler(&mut self, event: crate::app::AppMessage) {
        match event {
            crate::app::AppMessage::StartupAircraft => self.throw_initial_switches(),
            _ => {}
        }
    }
    fn wait_canopy_closed(&mut self, event: crate::app::AppMessage) {}
    fn wait_engine_started(&mut self, event: crate::app::AppMessage) {}
    fn done(&mut self, event: crate::app::AppMessage) {}

    fn throw_initial_switches(&self) {
        let _ = self
            .to_dcs_gamegui
            .send(|lua| {
                let switches_to_start = [
                    Switch::CanopyClose,
                    Switch::FuelPump1,
                    Switch::FuelPump3,
                    Switch::FuelPumpDrain,
                    Switch::BatteryOn,
                    Switch::BatteryHeat,
                    Switch::AcGenerator,
                    Switch::DcGenerator,
                    Switch::SprdPower,
                    Switch::SprdDropPower,
                    Switch::Po750Inverter1,
                    Switch::Po750Inverter2,
                    Switch::ApuPower,
                    Switch::FireExtinguisherPower,
                ];

                for s in switches_to_start {
                    let _ = dcs::mig21bis::set_switch(lua, s);
                }
            })
            .wait();
    }
}
