//! Just enough parsing to parse the output of ltl2ba/ltl3ba

use nom::{
    branch::alt, bytes::complete::tag, character::complete::*, combinator::map_res,
    error::ErrorKind, multi::*, sequence::*, AsChar, Finish, IResult, InputTakeAtPosition,
};

use super::LogicPredicate;

impl LogicPredicate {
    pub fn from_promela_predicate(input: &str) -> Result<Self, nom::error::Error<&str>> {
        parse_promela(input)
    }
}

fn parse_prop(input: &str) -> IResult<&str, LogicPredicate> {
    let (rest, name) = input.split_at_position1_complete(
        |item| !item.is_alphanum() && !['_'].contains(&item),
        ErrorKind::AlphaNumeric,
    )?;

    let name = name.to_string();
    Ok((rest, LogicPredicate::Prop(name.clone())))
}

fn parse_one(input: &str) -> IResult<&str, LogicPredicate> {
    map_res(tag("1"), |_| {
        Result::<_, nom::error::Error<&str>>::Ok(LogicPredicate::True)
    })(input)
}

fn parse_atom(input: &str) -> IResult<&str, LogicPredicate> {
    alt((parse_one, parse_prop))(input)
}

fn parse_neg(input: &str) -> IResult<&str, LogicPredicate> {
    map_res(preceded(char('!'), parse_prop), |s| {
        Result::<_, nom::error::Error<&str>>::Ok(LogicPredicate::Not(Box::new(s)))
    })(input)
}

fn parse_conj(input: &str) -> IResult<&str, LogicPredicate> {
    let (rest, vs) = separated_list1(tag(" && "), alt((parse_neg, parse_atom)))(input)?;
    Ok((
        rest,
        vs.into_iter()
            .reduce(|a, b| LogicPredicate::And(Box::new(a), Box::new(b)))
            .unwrap(),
    ))
}

fn parse_parens(input: &str) -> IResult<&str, LogicPredicate> {
    delimited(char('('), parse_conj, char(')'))(input)
}

fn parse_disj(input: &str) -> IResult<&str, LogicPredicate> {
    let (rest, vs) = separated_list1(tag(" || "), parse_parens)(input)?;
    Ok((
        rest,
        vs.into_iter()
            .reduce(|a, b| LogicPredicate::Or(Box::new(a), Box::new(b)))
            .unwrap(),
    ))
}

fn parse_promela(input: &str) -> Result<LogicPredicate, nom::error::Error<&str>> {
    let (rest, expr) = parse_disj(input).finish()?;
    assert!(rest.is_empty());
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_predicate_test() {
        println!("{}", parse_promela("(a && !b)").unwrap());
        println!("{}", parse_promela("(a && !b) || (c)").unwrap());
    }
}
