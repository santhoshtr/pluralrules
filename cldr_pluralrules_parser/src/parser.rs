use super::ast::*;
use nom::{
    IResult,
    Parser,
    branch::alt,
    //error::context,
    bytes::complete::tag,
    character::complete::{digit1, one_of, space0, space1},
    combinator::{map, map_res, opt},
    multi::{separated_list0, separated_list1},
    sequence::{preceded, separated_pair},
};

fn value(i: &str) -> IResult<&str, Value> {
    map_res(digit1, |s: &str| s.parse::<usize>().map(Value)).parse(i)
}

fn range(i: &str) -> IResult<&str, Range> {
    map(
        separated_pair(value, tag(".."), value),
        |(lower_val, upper_val)| Range {
            lower_val,
            upper_val,
        },
    )
    .parse(i)
}

fn range_list_item(i: &str) -> IResult<&str, RangeListItem> {
    alt((
        map(range, RangeListItem::Range),
        map(value, RangeListItem::Value),
    ))
    .parse(i)
}

fn range_list(i: &str) -> IResult<&str, RangeList> {
    map(
        separated_list0((space0, tag(","), space0), range_list_item),
        RangeList,
    )
    .parse(i)
}

fn operand(i: &str) -> IResult<&str, Operand> {
    map(one_of("nivwft"), |c| match c {
        'n' => Operand::N,
        'i' => Operand::I,
        'v' => Operand::V,
        'w' => Operand::W,
        'f' => Operand::F,
        't' => Operand::T,
        _ => unreachable!(),
    })
    .parse(i)
}

fn mod_expression(i: &str) -> IResult<&str, Option<Modulo>> {
    opt(map(
        preceded((space0, alt((tag("mod"), tag("%"))), space1), value),
        Modulo,
    ))
    .parse(i)
}

fn expression(i: &str) -> IResult<&str, Expression> {
    map((operand, mod_expression), |(operand, modulus)| Expression {
        operand,
        modulus,
    })
    .parse(i)
}

fn relation_operator(i: &str) -> IResult<&str, Operator> {
    alt((
        map(tag("="), |_| Operator::EQ),
        map(tag("!="), |_| Operator::NotEQ),
        map((tag("is"), space1, opt(tag("not"))), |(_, _, n)| {
            if n.is_some() {
                Operator::IsNot
            } else {
                Operator::Is
            }
        }),
        map(tag("in"), |_| Operator::In),
        map(
            (
                tag("not"),
                space1,
                alt((
                    map(tag("in"), |_| Operator::NotIn),
                    map(tag("within"), |_| Operator::NotWithin),
                )),
            ),
            |(_, _, v)| v,
        ),
        map(tag("within"), |_| Operator::Within),
    ))
    .parse(i)
}

fn relation(i: &str) -> IResult<&str, Relation> {
    map(
        (expression, space0, relation_operator, space0, range_list),
        |(expression, _, operator, _, range_list)| Relation {
            expression,
            operator,
            range_list,
        },
    )
    .parse(i)
}

fn and_condition(i: &str) -> IResult<&str, AndCondition> {
    map(
        separated_list1((space1, tag("and"), space1), relation),
        AndCondition,
    )
    .parse(i)
}

fn decimal_value(i: &str) -> IResult<&str, DecimalValue> {
    map(
        (value, opt(preceded(tag("."), value))),
        |(integer, decimal)| DecimalValue { integer, decimal },
    )
    .parse(i)
}

fn sample_range(i: &str) -> IResult<&str, SampleRange> {
    map(
        (
            decimal_value,
            opt(preceded((space0, tag("~"), space0), decimal_value)),
        ),
        |(lower_val, upper_val)| SampleRange {
            lower_val,
            upper_val,
        },
    )
    .parse(i)
}

fn sample_list(i: &str) -> IResult<&str, SampleList> {
    map(
        (
            separated_list1((space0, tag(","), space0), sample_range),
            opt(preceded(
                (space0, tag(","), space0),
                alt((tag("..."), tag("…"))),
            )),
        ),
        |(l, ellipsis)| SampleList {
            sample_ranges: l,
            ellipsis: ellipsis.is_some(),
        },
    )
    .parse(i)
}

fn samples(i: &str) -> IResult<&str, Option<Samples>> {
    map(
        (
            opt(preceded((space1, tag("@integer"), space1), sample_list)),
            opt(preceded((space1, tag("@decimal"), space1), sample_list)),
        ),
        |(integer, decimal)| {
            if integer.is_some() || decimal.is_some() {
                Some(Samples { integer, decimal })
            } else {
                None
            }
        },
    )
    .parse(i)
}

pub fn parse_rule(i: &str) -> IResult<&str, Rule> {
    map((parse_condition, samples), |(condition, samples)| Rule {
        condition,
        samples,
    })
    .parse(i)
}

pub fn parse_condition(i: &str) -> IResult<&str, Condition> {
    // We need to handle empty input and/or input that is empty until sample.
    if i.trim().is_empty() {
        return IResult::Ok(("", Condition(vec![])));
    }

    if i.trim().starts_with("@") {
        return IResult::Ok(("", Condition(vec![])));
    }
    map(
        separated_list1((space1, tag("or"), space1), and_condition),
        Condition,
    )
    .parse(i)
}
