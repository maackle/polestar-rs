/// A 4-way traffic intersection with traffic lights.
pub struct Intersection {
    /// Traffic lights for north-south direction
    ns_light: TrafficLight,
    /// Traffic lights for east-west direction  
    ew_light: TrafficLight,
    /// Current state of the intersection
    state: IntersectionState,
}

/// Represents a traffic light with three states
#[derive(Debug, PartialEq, Clone)]
pub enum TrafficLight {
    Red,
    Yellow,
    Green,
}

/// Represents the current state of traffic flow
#[derive(Debug, PartialEq)]
pub enum IntersectionState {
    /// Traffic is flowing in the north-south direction (north-south has green light, east-west has red)
    NorthSouthFlow,
    /// Traffic is flowing in the east-west direction (east-west has green light, north-south has red)
    EastWestFlow,
    /// The intersection is in the process of switching directions (one direction has yellow light)
    Transitioning,
}

impl Default for Intersection {
    /// Create a new intersection, initially with north-south traffic flowing
    fn default() -> Self {
        Intersection {
            ns_light: TrafficLight::Green,
            ew_light: TrafficLight::Red,
            state: IntersectionState::NorthSouthFlow,
        }
    }
}

impl Intersection {
    /// Create a new intersection, initially with north-south traffic flowing
    pub fn new() -> Self {
        Self::default()
    }

    /// Change the traffic flow from one direction to another
    pub fn switch_flow(&mut self) {
        match self.state {
            IntersectionState::NorthSouthFlow => {
                self.ns_light = TrafficLight::Yellow;
                self.state = IntersectionState::Transitioning;
            }
            IntersectionState::EastWestFlow => {
                self.ew_light = TrafficLight::Yellow;
                self.state = IntersectionState::Transitioning;
            }
            IntersectionState::Transitioning => {
                if self.ns_light == TrafficLight::Yellow {
                    self.ns_light = TrafficLight::Red;
                    self.ew_light = TrafficLight::Green;
                    self.state = IntersectionState::EastWestFlow;
                } else {
                    self.ew_light = TrafficLight::Red;
                    self.ns_light = TrafficLight::Green;
                    self.state = IntersectionState::NorthSouthFlow;
                }
            }
        }
    }

    /// Get the current state of the traffic lights
    pub fn get_light_states(&self) -> (TrafficLight, TrafficLight) {
        (self.ns_light.clone(), self.ew_light.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_intersection() {
        let intersection = Intersection::new();
        assert_eq!(intersection.ns_light, TrafficLight::Green);
        assert_eq!(intersection.ew_light, TrafficLight::Red);
        assert_eq!(intersection.state, IntersectionState::NorthSouthFlow);
    }

    #[test]
    fn test_switch_flow() {
        let mut intersection = Intersection::new();

        // Initial state to transitioning
        intersection.switch_flow();
        assert_eq!(intersection.ns_light, TrafficLight::Yellow);
        assert_eq!(intersection.ew_light, TrafficLight::Red);
        assert_eq!(intersection.state, IntersectionState::Transitioning);

        // Transitioning to east-west flow
        intersection.switch_flow();
        assert_eq!(intersection.ns_light, TrafficLight::Red);
        assert_eq!(intersection.ew_light, TrafficLight::Green);
        assert_eq!(intersection.state, IntersectionState::EastWestFlow);
    }
}
