use crate::app::FsmMessage;
use crate::dcs;
use egui_backend::egui;
use mlua::prelude::LuaResult;
use mlua::Lua;
use offload::TaskSender;
use strum::IntoStaticStr;

use super::get_cockpit_param;

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
    EngineStart,
    EngineStartLight,
    Gyro1,
    Gyro2,
    SrzoPower,
    SauPower,
    SauPitchPower,
    TrimmerPower,
    NoseconePower,
    EmergencyHydroPump,
    KppMainEmergencyToggle,
    NppPower,
    RadAltPower,
    AspPower,
    MissileHeatPower,
    MissileLaunchPower,
    InboardPylonPower,
    OutboardPylonPower,
    GunPower,
    GunCameraPower,
    FlightRecorderPower,
    RadioPower,
    ArkPower,
    RadarPower,
    SpoPower,
    Srzo81Power,
    SodPower,
    SprdCover,
    PipperEnable,
    FixedNetEnable,
    GunPyro1,
    GunPyro2,
    GunPyro3,
    WeaponModeAaAg,
    GuidedMissileMode,
    WeaponSelect,
    NppAdjust,
    NumSwitches,
}
type St = crate::dcs::SwitchType;

type Info = dcs::SwitchInfo<Switch>;

pub const SWITCH_INFO_MAP: [Info; Switch::NumSwitches as usize] = [
    Info::new(Switch::FuelPump1, 4, 3011, 160, St::Toggle),
    Info::new(Switch::FuelPump3, 4, 3010, 159, St::Toggle),
    Info::new(Switch::FuelPumpDrain, 4, 3012, 161, St::Toggle),
    Info::new(Switch::BatteryOn, 1, 3001, 165, St::Toggle),
    Info::new(Switch::BatteryHeat, 1, 3002, 155, St::Toggle),
    Info::new(Switch::AcGenerator, 2, 3004, 169, St::Toggle),
    Info::new(Switch::DcGenerator, 1, 3003, 166, St::Toggle),
    Info::new(Switch::SprdPower, 48, 3106, 167, St::Toggle),
    Info::new(Switch::SprdDropPower, 48, 3107, 168, St::Toggle),
    Info::new(Switch::Po750Inverter1, 2, 3005, 153, St::Toggle),
    Info::new(Switch::Po750Inverter2, 2, 3006, 154, St::Toggle),
    Info::new(Switch::ApuPower, 3, 3014, 302, St::Toggle),
    Info::new(Switch::FireExtinguisherPower, 53, 3025, 303, St::Toggle),
    Info::new(Switch::ThrottleStopLock, 3, 3238, 616, St::Toggle),
    Info::new(Switch::CanopyOpen, 43, 3152, 375, St::Momentary),
    Info::new(Switch::CanopyClose, 43, 3194, 385, St::Momentary),
    Info::new(Switch::CanopyLock, 43, 3151, 329, St::Toggle),
    Info::new(Switch::CanopySeal, 43, 3150, 328, St::Toggle),
    Info::new(Switch::EngineStart, 3, 3016, 289, St::Momentary),
    Info::new(Switch::EngineStartLight, 0, 0, 509, St::Indicator),
    Info::new(Switch::Gyro1, 21, 3008, 162, St::Toggle),
    Info::new(Switch::Gyro2, 21, 3009, 163, St::Toggle),
    Info::new(Switch::SrzoPower, 38, 3087, 188, St::Toggle),
    Info::new(Switch::SauPower, 8, 3064, 179, St::Toggle),
    Info::new(Switch::SauPitchPower, 8, 3065, 180, St::Toggle),
    Info::new(Switch::TrimmerPower, 9, 3131, 172, St::Toggle),
    Info::new(Switch::NoseconePower, 17, 3133, 170, St::Toggle),
    Info::new(Switch::EmergencyHydroPump, 44, 3137, 171, St::Toggle),
    Info::new(Switch::KppMainEmergencyToggle, 28, 3139, 177, St::Toggle),
    Info::new(Switch::NppPower, 23, 3142, 178, St::Toggle),
    Info::new(Switch::RadAltPower, 33, 3145, 175, St::Toggle),
    Info::new(Switch::AspPower, 41, 3155, 186, St::Toggle),
    Info::new(Switch::MissileHeatPower, 42, 3167, 181, St::Toggle),
    Info::new(Switch::MissileLaunchPower, 42, 3168, 182, St::Toggle),
    Info::new(Switch::InboardPylonPower, 42, 3169, 183, St::Toggle),
    Info::new(Switch::OutboardPylonPower, 42, 3170, 184, St::Toggle),
    Info::new(Switch::GunPower, 42, 3171, 185, St::Toggle),
    Info::new(Switch::GunCameraPower, 42, 3172, 187, St::Toggle),
    Info::new(Switch::FlightRecorderPower, 49, 3209, 193, St::Toggle),
    Info::new(Switch::RadioPower, 22, 3041, 173, St::Toggle),
    Info::new(Switch::ArkPower, 24, 3047, 174, St::Toggle),
    Info::new(Switch::RadarPower, 40, 3094, 205, St::MultiToggle),
    Info::new(Switch::SpoPower, 37, 3083, 202, St::Toggle),
    Info::new(Switch::Srzo81Power, 38, 3089, 346, St::Toggle),
    Info::new(Switch::SodPower, 39, 3090, 200, St::Toggle),
    Info::new(Switch::SprdCover, 48, 3109, 317, St::Toggle),
    Info::new(Switch::PipperEnable, 41, 3160, 249, St::Toggle),
    Info::new(Switch::FixedNetEnable, 41, 3161, 250, St::Toggle),
    Info::new(Switch::GunPyro1, 42, 3185, 232, St::Momentary),
    Info::new(Switch::GunPyro2, 42, 3186, 233, St::Momentary),
    Info::new(Switch::GunPyro3, 42, 3187, 234, St::Momentary),
    Info::new(Switch::WeaponModeAaAg, 42, 3183, 230, St::Toggle),
    Info::new(Switch::GuidedMissileMode, 42, 3184, 231, St::MultiToggle),
    Info::new(Switch::WeaponSelect, 42, 3188, 235, St::MultiToggle),
    Info::new(Switch::NppAdjust, 23, 3143, 258, St::Momentary),
];

#[derive(Clone, PartialEq, Debug)]
enum StartupState {
    ColdDark,
    WaitCanopyClosed,
    WaitEngineStartBegun,
    WaitEngineStartComplete,
    Done,
}

fn get_switch_info(s: Switch) -> &'static Info {
    &SWITCH_INFO_MAP[s as usize]
}

fn toggle_switch(lua: &Lua, s: Switch) -> LuaResult<()> {
    let info = get_switch_info(s);
    dcs::perform_click(lua, info.device_id, info.command, 1.0)
}

pub fn set_switch_state(lua: &Lua, s: Switch, state: f32) -> LuaResult<()> {
    let info = get_switch_info(s);
    dcs::perform_click(lua, info.device_id, info.command, state)
}

pub fn get_switch_state(lua: &Lua, s: Switch) -> LuaResult<f32> {
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

fn poll_argument(to_gamegui: &TaskSender<Lua>, switch: Switch) -> Result<f32, crate::Error> {
    to_gamegui
        .send(move |lua| get_switch_state(lua, switch))
        .wait()
        .map_err(|_| crate::Error::CommError)?
        .map_err(|e| crate::Error::LuaError(e))
}

fn handle_polling_err(r: &Result<f32, crate::Error>) {
    if let Err(err) = r {
        match err {
            crate::Error::LuaError(e) => {
                log::warn!(
                    "Lua error encountered when polling Engine Start Light {:?}",
                    e
                )
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fsm {
    state: StartupState,
    to_dcs_gamegui: TaskSender<Lua>,
    to_dcs_export: TaskSender<Lua>,
    gui: crate::gui::TxHandle,
}

impl dcs::AircraftFsm for Fsm {
    fn run_fsm(&mut self, event: FsmMessage) {
        match self.state {
            StartupState::ColdDark => self.cold_dark_handler(event),
            StartupState::WaitCanopyClosed => self.wait_canopy_closed(event),
            StartupState::WaitEngineStartBegun => self.wait_engine_start_begun(event),
            StartupState::WaitEngineStartComplete => self.wait_engine_start_complete(event),
            StartupState::Done => self.done(event),
        }
    }
}

impl Fsm {
    pub fn new(
        to_dcs_gamegui: TaskSender<Lua>,
        to_dcs_export: TaskSender<Lua>,
        gui: crate::gui::TxHandle,
    ) -> Self {
        Self {
            state: StartupState::ColdDark,
            to_dcs_gamegui,
            to_dcs_export,
            gui,
        }
    }
    fn cold_dark_handler(&mut self, event: crate::app::FsmMessage) {
        match event {
            crate::app::FsmMessage::StartupAircraft => {
                // this should cause the progress bar to begin animating
                self.gui.set_startup_progress(0.001);
                self.gui.set_startup_text("Setting up initial switches");
                self.throw_initial_switches();
                self.gui.set_startup_progress(0.05);
                self.gui.set_startup_text("Waiting for canopy to close");

                self.state = StartupState::WaitCanopyClosed;
            }
            _ => {}
        }
    }
    fn wait_canopy_closed(&mut self, _event: crate::app::FsmMessage) {
        let r = self
            .to_dcs_export
            .send(|lua| get_cockpit_param(lua, "BASE_SENSOR_CANOPY_POS"))
            .wait();
        if let Err(_) = r {
            return;
        }

        let lua_result = r.unwrap();
        if let Err(e) = lua_result {
            log::warn!("Polling canopy failed {:?}", e);
            return;
        }

        let cockpit_state = lua_result.unwrap();
        if cockpit_state == 0.0 as f32 {
            self.gui.set_startup_progress(0.1);
            self.gui.set_startup_text("Sealing canopy");
            let _ = self
                .to_dcs_gamegui
                .send(|lua| {
                    let _ = set_switch(lua, Switch::CanopyLock);
                    let _ = set_switch(lua, Switch::CanopySeal);
                    let _ = set_switch_state(lua, Switch::EngineStart, 1.0);
                })
                .wait();
            self.gui.set_startup_progress(0.18);
            self.gui
                .set_startup_text("Waiting for engine start sequence");
            self.state = StartupState::WaitEngineStartBegun;
        }
    }

    fn wait_engine_start_begun(&mut self, _event: crate::app::FsmMessage) {
        let result = poll_argument(&self.to_dcs_gamegui, Switch::EngineStartLight);
        if result.is_err() {
            handle_polling_err(&result);
            return;
        }
        let light = result.unwrap();
        if light < 0.9 {
            return;
        }
        self.gui.set_startup_text("Starting up systems");
        self.to_dcs_gamegui
            .send(|lua| set_switch_state(lua, Switch::EngineStart, 0.0));
        self.gui.set_startup_progress(0.2);

        self.throw_post_engine_start_switches();
        self.gui.set_startup_progress(0.22);
        self.gui
            .set_startup_text("Waiting for engine start sequence to complete");
        self.state = StartupState::WaitEngineStartComplete;
    }

    fn wait_engine_start_complete(&mut self, _event: crate::app::FsmMessage) {
        let result = poll_argument(&self.to_dcs_gamegui, Switch::EngineStartLight);
        if result.is_err() {
            handle_polling_err(&result);
            return;
        }
        let light = result.unwrap();
        if light > 0.1 {
            return;
        }
        self.gui.set_startup_text("Waiting for NPP adjust");
        self.gui.set_startup_progress(0.8);
        // this should be moved into a separate state and made nonblocking, but for now
        // just block the thread for 5 seconds while it aligns.
        self.run_npp_adjust();

        self.gui.set_startup_progress(1.0);
        self.gui.set_startup_text("DONE");
        self.state = StartupState::Done;
    }

    fn done(&mut self, _event: crate::app::FsmMessage) {}

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
                    Switch::ThrottleStopLock,
                ];

                for s in switches_to_start {
                    let _ = set_switch(lua, s);
                }
            })
            .wait();
    }

    fn throw_post_engine_start_switches(&self) {
        let _ = self
            .to_dcs_gamegui
            .send(|lua| {
                let switches = [
                    Switch::Gyro1,
                    Switch::Gyro2,
                    Switch::SrzoPower,
                    Switch::SauPower,
                    Switch::SauPitchPower,
                    Switch::TrimmerPower,
                    Switch::NoseconePower,
                    Switch::EmergencyHydroPump,
                    Switch::KppMainEmergencyToggle,
                    Switch::NppPower,
                    Switch::RadAltPower,
                    Switch::AspPower,
                    Switch::MissileHeatPower,
                    Switch::MissileLaunchPower,
                    Switch::InboardPylonPower,
                    Switch::OutboardPylonPower,
                    Switch::GunPower,
                    Switch::FlightRecorderPower,
                    Switch::RadioPower,
                    Switch::ArkPower,
                    Switch::SpoPower,
                    Switch::PipperEnable,
                    Switch::FixedNetEnable,
                    Switch::WeaponModeAaAg,
                    Switch::Srzo81Power,
                    Switch::SodPower,
                    Switch::SprdCover,
                ];
                for s in switches {
                    let _ = set_switch(lua, s);
                }
                let _ = set_switch_state(lua, Switch::WeaponSelect, 0.7);
                let _ = set_switch_state(lua, Switch::GuidedMissileMode, 1.0);
                let _ = set_switch_state(lua, Switch::RadarPower, 0.5);
                let _ = set_switch_state(lua, Switch::GunPyro1, 1.0);
            })
            .wait();
        let _ = self
            .to_dcs_gamegui
            .send(|lua| set_switch_state(lua, Switch::GunPyro1, 0.0))
            .wait();
    }

    fn run_npp_adjust(&self) {
        let _ = self
            .to_dcs_gamegui
            .send(|lua| set_switch_state(lua, Switch::NppAdjust, 1.0))
            .wait();

        std::thread::sleep(std::time::Duration::from_secs(6));
        let _ = self
            .to_dcs_gamegui
            .send(|lua| set_switch_state(lua, Switch::NppAdjust, 0.0))
            .wait();
    }
}

pub fn make_debug_widget(ui: &mut egui::Ui, strings: &mut Vec<String>, tx: &TaskSender<Lua>) {
    egui::Grid::new("debug_grid").show(ui, |ui| {
        for (ii, &ref switch_info) in SWITCH_INFO_MAP.iter().enumerate() {
            let s: &'static str = switch_info.switch.into();
            ui.label(s);
            let val = &mut (strings[ii]);
            if ui.button("Set").clicked() {
                let result = val.parse::<f32>();
                if let Ok(state) = result {
                    let _ = tx.send(move |lua| set_switch_state(lua, switch_info.switch, state));
                }
            }
            if ui.button("Get").clicked() {
                let result = tx
                    .send(|lua| get_switch_state(lua, switch_info.switch))
                    .wait();
                if let Ok(state) = result {
                    val.replace_range(.., state.unwrap_or_default().to_string().as_str());
                }
            }
            ui.add(egui::TextEdit::singleline(val));
            ui.end_row();
        }
    });
}
