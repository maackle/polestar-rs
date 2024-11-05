use proptest::prelude::*;
use proptest::test_runner::TestRunner;

use polestar::prelude::*;
use proptest_derive::Arbitrary;

type Temp = i32;
type Hum = u8;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Arbitrary)]
struct TempSetting {
    target: Temp,
    tolerance: u8,
}

impl TempSetting {
    fn new(target: Temp, tolerance: u8) -> Self {
        Self { target, tolerance }
    }

    fn lo(&self) -> Temp {
        self.target - self.tolerance as i32 / 2
    }

    fn hi(&self) -> Temp {
        self.target + self.tolerance as i32 / 2
    }
}

#[derive(Clone, Debug, Arbitrary)]
struct Instrument {
    setting: TempSetting,
    min: InstrumentReading,
    max: InstrumentReading,
    current: InstrumentReading,
}

impl Instrument {
    fn new(setting: TempSetting) -> Self {
        Self {
            setting,
            min: InstrumentReading { temp: 0, hum: 0 },
            max: InstrumentReading { temp: 0, hum: 0 },
            current: InstrumentReading { temp: 0, hum: 0 },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Arbitrary)]
struct InstrumentReading {
    temp: Temp,
    hum: Hum,
}

#[derive(Clone, Debug, PartialEq, Eq, Arbitrary)]
struct Thermostat {
    setting: TempSetting,
    state: ThermostatState,
}

impl Thermostat {
    pub fn new(setting: TempSetting) -> Self {
        Self {
            setting,
            state: ThermostatState::Idle,
        }
    }

    fn set(self, state: ThermostatState) -> Self {
        Self {
            setting: self.setting,
            state,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Arbitrary)]
enum ThermostatState {
    Idle,
    Heating,
    Cooling,
}

enum HygrometerFsm {
    Dry,
    Nice,
    Humid,
}

impl Fsm for Thermostat {
    type Event = Temp;
    type Fx = ();
    type Error = Infallible;

    fn transition(mut self, temp: Self::Event) -> Result<(Self, Self::Fx), Self::Error> {
        if temp < self.setting.lo() {
            self.state = ThermostatState::Heating;
        } else if temp > self.setting.hi() {
            self.state = ThermostatState::Cooling;
        } else {
            self.state = ThermostatState::Idle;
        }
        Ok((self, ()))
    }
}

// impl FsmFx for Thermostat {
//     type Event = Temp;
//     type Fx = ();

//     fn transition(mut self, temp: Self::Event) -> (Self, Self::Fx) {
//         if temp < self.setting.lo() {
//             self.state = ThermostatState::Heating;
//         } else if temp > self.setting.hi() {
//             self.state = ThermostatState::Cooling;
//         } else {
//             self.state = ThermostatState::Idle;
//         }
//         (self, ())
//     }
// }

impl Projection<Thermostat> for Instrument {
    type Event = InstrumentReading;

    fn apply(&mut self, event: Self::Event) {
        self.current = event;
        if self.min.temp > event.temp {
            self.min.temp = event.temp;
        }
        if self.max.temp < event.temp {
            self.max.temp = event.temp;
        }
        if self.min.hum > event.hum {
            self.min.hum = event.hum;
        }
        if self.max.hum < event.hum {
            self.max.hum = event.hum;
        }
    }

    fn map_event(&self, event: Self::Event) -> Option<Temp> {
        Some(event.temp)
    }

    fn map_state(&self) -> Option<Thermostat> {
        let s = Thermostat {
            setting: self.setting,
            state: ThermostatState::Idle,
        };
        Some(s.transition_(self.current.temp).unwrap())
    }

    fn gen_event(&self, g: &mut impl Generator, temp: Temp) -> InstrumentReading {
        InstrumentReading {
            temp,
            hum: g.generate().unwrap(),
        }
    }

    fn gen_state(&self, g: &mut impl Generator, state: Thermostat) -> Self {
        let lo = self.setting.lo();
        let hi = self.setting.hi();
        let temp: Temp = match state.state {
            ThermostatState::Idle => g.generate_with(lo..=hi).unwrap(),
            ThermostatState::Cooling => g.generate_with(hi + 1..).unwrap(),
            ThermostatState::Heating => g.generate_with(..lo).unwrap(),
        };
        let mut current: InstrumentReading = g.generate().unwrap();
        current.temp = temp;
        let mut new = self.clone();
        new.current = current;
        new
    }
}

proptest! {
    #[test]
    fn test_thermostat(mut instrument: Instrument, event: InstrumentReading) {
        let mut r = TestRunner::default();
        instrument.clone().test_invariants(&mut r, event);
        // instrument = instrument.apply(event);
    }
}
