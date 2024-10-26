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

    fn transition(&mut self, temp: Self::Event) {
        if temp < self.setting.lo() {
            self.state = ThermostatState::Heating;
        } else if temp > self.setting.hi() {
            self.state = ThermostatState::Cooling;
        } else {
            self.state = ThermostatState::Idle;
        }
    }
}

impl Projection<Thermostat> for Instrument {
    type Event = InstrumentReading;

    fn apply(mut self, event: Self::Event) -> Self {
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
        self
    }

    fn map_event(&self, event: Self::Event) -> Temp {
        event.temp
    }

    fn map_state(&self) -> Thermostat {
        let mut s = Thermostat {
            setting: self.setting,
            state: ThermostatState::Idle,
        };
        s.transition(self.current.temp);
        s
    }

    fn gen_event(&self, g: &mut impl Generate, temp: Temp) -> InstrumentReading {
        InstrumentReading {
            temp,
            hum: g.generate().unwrap(),
        }
    }

    fn gen_state(&self, g: &mut impl Generate, state: Thermostat) -> Self {
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
        instrument.clone().test_invariants(&mut r, event.clone());
        // instrument = instrument.apply(event);
    }
}
