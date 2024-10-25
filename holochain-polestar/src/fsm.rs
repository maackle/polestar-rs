pub struct Fsm<State, Event, Meta = ()> {
    state: State,
    event: Event,
    meta: Meta,
}
