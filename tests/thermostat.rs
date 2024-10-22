#![cfg(feature = "testing")]

use proptest::prelude::*;
use proptest::test_runner::TestRunner;

use polestar::{ArbitraryExt, Fsm, Projection, ProjectionTests};
use proptest_derive::Arbitrary;

fn main() {}

type Temp = i32;
type Hum = u8;

#[derive(Clone, Debug, Arbitrary)]
struct Instrument {
    range: (Temp, Temp),
    min: InstrumentReading,
    max: InstrumentReading,
    current: InstrumentReading,
}

impl Instrument {
    fn new(range: (Temp, Temp)) -> Self {
        Self {
            range,
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
    range: (Temp, Temp),
    state: ThermostatState,
}

impl Thermostat {
    pub fn new(range: (Temp, Temp)) -> Self {
        Self {
            range,
            state: ThermostatState::Idle,
        }
    }

    fn set(self, state: ThermostatState) -> Self {
        Self {
            range: self.range,
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
    type Transition = Temp;

    fn transition(self, temp: Self::Transition) -> Self {
        if temp < self.range.0 {
            self.set(ThermostatState::Heating)
        } else if temp > self.range.1 {
            self.set(ThermostatState::Cooling)
        } else {
            self.set(ThermostatState::Idle)
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
        Thermostat {
            range: self.range,
            state: ThermostatState::Idle,
        }
        .transition(self.current.temp)
    }

    fn gen_event(&self, runner: &mut TestRunner, temp: Temp) -> InstrumentReading {
        // let temp = match transition {
        //     Temp::Idle => runner.generate_with(self.range),
        //     Temp::Heating => runner.generate_with(self.range.end()..),
        //     Temp::Cooling => runner.generate_with(..self.range.start()),
        // };
        InstrumentReading {
            temp,
            hum: runner.arbitrary().unwrap(),
        }
    }

    fn gen_state(&self, runner: &mut TestRunner, state: Thermostat) -> Self {
        Self {
            range: self.range,
            min: self.min,
            max: self.max,
            current: runner.arbitrary().unwrap(),
        }
    }
}

#[test]
fn hi() {}

proptest! {
    #[test]
    fn test_thermostat(instrument: Instrument, events: Vec<InstrumentReading>) {
        let mut r = TestRunner::default();
        let mut state = instrument.gen_state(&mut r, Thermostat::new((70, 80)));
        for event in events {
            state.clone().test_invariants(&mut r, event.clone());
            state = state.apply(event);
        }
    }
}
