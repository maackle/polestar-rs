use mcltl_lib::{
    buchi::{self, Buchi},
    ltl::{automata, expression::*},
};

use super::{Machine, TransitionResult};

pub fn buchi_from_ltl(mut ltl_property: LTLExpression) -> Buchi {
    // let mut ltl_property = LTLExpression::try_from(property).expect("cannot convert try form");
    ltl_property.rewrite();
    let nnf_ltl_property = put_in_nnf(ltl_property);

    let nodes = automata::create_graph(nnf_ltl_property.clone());

    let gbuchi_property = buchi::extract_buchi(nodes, nnf_ltl_property);

    let buchi_property: Buchi = gbuchi_property.into();

    buchi_property
}

pub struct Predicates<M: Machine>(Vec<Box<dyn Fn(&M::State) -> bool>>);

pub struct BuchiChecker<M> {
    buchi: Buchi,
    machine: M,
    // predicates: Predicates,
}

impl<M> Machine for BuchiChecker<M>
where
    M: Machine,
{
    type State = State<M>;
    type Action = M::Action;

    fn transition(&self, state: Self::State, action: Self::Action) -> TransitionResult<Self> {
        todo!()
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        self.machine.is_terminal(&state.state)
    }
}

pub struct State<M: Machine> {
    state: M::State,
    buchi_indices: Vec<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asdgi() {
        use mcltl_lib::ltl::expression::LTLExpression as L;
        // let ltl_property = LTLExpression::try_from("G a").unwrap();
        let ltl_property = L::G(Box::new(L::F(Box::new(L::Literal("b".to_string())))));
        let buchi = buchi_from_ltl(ltl_property);
        dbg!(&buchi);
    }
}
