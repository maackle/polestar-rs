//! A mechanism for working with LTL formulae.
//!
//! This module is the simplest thing I could cobble together to get LTL-to-Buchi
//! working. It's not a great long-term solution, but it's a start.
//! Eventually, this will be replaced by a native LTL-to-Buchi implementation,
//! rather than using `ltl3ba` as a command-line tool, which will allow working
//! with Proposition types directly and removing the need for an intermediate
//! string representation of LTL formulae. There will also be some macros for
//! expressing propositions within LTL statements directly.
//!
//! See this example to understand how the different parts fit together:
//!
//! ```
//! use polestar::prelude::*;
//!
//! struct Model;
//! #[derive(Clone, Debug, PartialEq, Eq, Hash)]
//! struct State(u8);
//! #[derive(Clone, Debug, PartialEq, Eq, Hash, exhaustive::Exhaustive)]
//! struct Action;
//!
//! impl Machine for Model {
//!     type State = State;
//!     type Action = Action;
//!     type Error = ();
//!     type Fx = ();
//!
//!     fn transition(&self, mut state: Self::State, action: Self::Action) -> TransitionResult<Self> {
//!         state.0 = (state.0 + 1) % 3;
//!         Ok((state, ()))
//!     }
//! }
//!
//!
//! #[derive(Clone, PartialEq, Eq, derive_more::Display)]
//! enum Prop {
//!     A,
//!     B,
//!     C,
//! }
//!
//!
//! impl Propositions<Prop> for Transition<Model> {
//!     fn eval(&self, prop: &Prop) -> bool {
//!         let Transition(s0, _action, _s1) = self;
//!         match *prop {
//!             Prop::A => s0.0 == 0,
//!             Prop::B => s0.0 == 1,
//!             Prop::C => s0.0 == 2,
//!         }
//!     }
//! }
//!
//!
//! let mut props = PropRegistry::<Prop>::empty();
//! let a = props.add(Prop::A).unwrap();
//! let b = props.add(Prop::B).unwrap();
//! let c = props.add(Prop::C).unwrap();
//!
//! let ltl = format!("G ({a} -> F {c})");
//!
//! Model.traverse([State(0)]).specced(props, &ltl).unwrap().model_check_report().unwrap();
//!
//! ```

use super::*;

/// A source of truth for finding the truth value of a proposition `P`.
pub trait EvaluatePropositions<P> {
    /// Evaluate the truth value of a proposition `P`.
    fn evaluate(&self, proposition: &P) -> bool;
}

/// A registry for atomic propositions, used to build up LTL formulae.
///
/// This mechanism is mostly a necessity because polestar leans on an external command-line tool,
/// `ltl3ba`, for incorporating LTL formulae into [`crate::model_checker::ModelChecker`],
/// and so the input must be passed as a string.
/// The `PropRegistry` provides a relatively easy way to map between a custom type `P`
/// representing propositions, and strings which are used in an LTL formula.
///
/// Essentially, when registering a [`P`], its Display representation is used as the
/// name of the proposition in the LTL formula. The purpose of this registry is to
/// remember the string name, and use it later to look up the [`P`], which is used
/// to actually evaluate the truth value of that proposition against your model states.
///
#[derive(Clone, Debug)]
pub struct PropositionRegistry<P>(HashMap<String, P>);

impl<P: Clone> PropositionMapping for PropositionRegistry<P> {
    type Proposition = P;

    fn map(&self, name: &str) -> Option<Self::Proposition> {
        self.0.get(name).cloned()
    }
}

impl<P> PropositionRegistry<P>
where
    P: Display + Clone + PartialEq,
{
    /// Create an empty [`PropRegistry`].
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    /// Create a [`PropRegistry`] pre-populated with a list of [`P`]s.
    pub fn new<'a, T: Into<P>>(ps: impl IntoIterator<Item = T>) -> Result<Self, String> {
        let mut props = Self::empty();

        for p in ps {
            props.add(p.into())?;
        }

        Ok(props)
    }

    /// Add a [`P`] to the registry, and return the string representation of the [`P`]
    /// which can be used in an LTL formula.
    pub fn add(&mut self, p: P) -> Result<String, String> {
        let disp = p.to_string();
        let name = disp
            .to_lowercase()
            .replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");
        let name = if name.starts_with(|ch: char| ch.is_numeric()) {
            format!("p_{name}")
        } else {
            name
        };

        if let Some(old) = self.0.insert(name.clone(), p.clone()) {
            if old != p {
                return Err(format!(
                    "Attempted to add to propmap with name collision: {disp} -> {name}"
                ));
            }
        }
        Ok(name)
    }
}

/// Maps the string name of a proposition back to its proper type.
///
/// Basically, the reverse of what the [`PropositionRegistry`] provides.
pub trait PropositionMapping {
    /// The type of the proposition.
    type Proposition;

    /// Map a string name back to its proper type.
    fn map(&self, name: &str) -> Option<Self::Proposition>;
}

impl PropositionMapping for () {
    type Proposition = String;

    fn map(&self, name: &str) -> Option<String> {
        Some(name.to_string())
    }
}

/// Simply groups together a [`Transition`], which can always evaluate
/// a proposition, and a [`PropositionMapping`], which maps the string
/// name of a proposition back to its proper type which can be evaluated
/// by the [`Transition`].
pub(crate) struct PropositionBindings<'p, M, P>
where
    M: Machine,
    P: PropositionMapping,
{
    pub(crate) props: &'p P,
    pub(crate) transition: Transition<M>,
}

impl<M, P> EvaluatePropositions<String> for PropositionBindings<'_, M, P>
where
    M: Machine,
    P: PropositionMapping,
    Transition<M>: EvaluatePropositions<P::Proposition>,
{
    fn evaluate(&self, prop: &String) -> bool {
        let name = self
            .props
            .map(prop)
            .unwrap_or_else(|| panic!("no closure for prop: {}", prop));
        self.transition.evaluate(&name)
    }
}

/// Helper function to join two logic statements via a conjunction ("&&")
///
/// ```
/// assert_eq!(conjoin(&["a", "b || c", "!d"]), "((a) && (b || c) && (!d))");
/// ```
pub fn conjoin<T: Display>(predicates: impl IntoIterator<Item = T>) -> String {
    predicates
        .into_iter()
        .map(|p| format!("({p})"))
        .join(" && ")
}
