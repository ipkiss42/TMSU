use nom::branch::*;
use nom::bytes::complete::*;
use nom::character::complete::*;
use nom::combinator::*;
use nom::multi::*;
use nom::sequence::*;
use nom::*;

use super::{
    AndExpression, ComparisonExpression, Expression, NotExpression, Operator, OrExpression,
    TagExpression,
};

pub(super) fn parse_whitespace(input: &str) -> IResult<&str, &str> {
    terminated(white0, eof)(input)
}

pub(super) fn parse_expr(input: &str) -> IResult<&str, Expression> {
    terminated(full_expr, eof)(input)
}

// Here is the approximative grammar used for the parsing:
//      FullExpr := Space* OrExpr Space*
//      OrExpr := AndExpr (("or"|"OR") AndExpr)*
//      AndExpr := AndOperand (("and"|"AND"|"") AndOperand)*
//      AndOperand := (ParensExpr|ComparisonExpr|NotExpr|TagExpr)
//      ParensExpr := "(" Space* FullExpr Space* ")"
//      NotExpr := ("not"|"NOT") (TagExpr|ComparisonExpr|ParensExpr)
//      TagExpr := TagName
//      ComparisonExpr := TagName Operator ValueName
//      TagName := TagChar+
//      ValueName := TagChar+
//      TagChar := EscapedChar|!SpecialChar
//      EscapedChar := '\' [any Unicode char]
//      SpecialChar := [SPECIAL_CHARS]
//      Space := [Any Unicode White_Space]
//
// The above grammar is approximative for two reasons:
// 1) Handling whitespace is tricky. For example, "a and b" is equivalent to "(a)and(b)" but not to
//    "a andb", so we cannot assume either that whitespace is present or missing around operators:
//    it depends on the operands themselves (and sometimes even on operators, e.g. "=" doesn't
//    require whitespace even without parentheses but "eq" does).
// 2) Some keywords are reserved, either for parsing reasons (e.g. operators) or other reasons
//    (e.g. VFS forbidden values). Here we only handle the first category, leaving it to the caller
//    to validate tag and value names. This allows more explicit error messages.
//
// Note: there is no such thing as an empty expression. You can use Option<Expression> instead.

/// Characters forbidden in tag/value names, unless escaped with '\'
const SPECIAL_CHARS: &str = r"\()!=<>";

/// Keywords forbidden for a tag/value name.
/// All these should also be checked by entities::validate_name_helper().
const RESERVED_KEYWORDS: &[&str] = &["", "not", "and", "or", "eq", "ne", "lt", "le", "gt", "ge"];

fn make_tag(name: &str) -> Expression {
    Expression::Tag(TagExpression {
        tag: name.to_owned(),
    })
}

fn make_not(expr: Expression) -> Expression {
    Expression::Not(NotExpression {
        operand: Box::new(expr),
    })
}

fn make_and(left: Expression, right: Expression) -> Expression {
    Expression::And(AndExpression {
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn make_or(left: Expression, right: Expression) -> Expression {
    Expression::Or(OrExpression {
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn make_comparison(tag_name: &str, op: &str, value_name: &str) -> Expression {
    let operator = match op {
        "eq" | "EQ" | "Eq" | "eQ" | "=" | "==" => Operator::Equal,
        "ne" | "NE" | "Ne" | "nE" | "!=" => Operator::Different,
        "lt" | "LT" | "Lt" | "lT" | "<" => Operator::LessThan,
        "le" | "LE" | "Le" | "lE" | "<=" => Operator::LessThanOrEqual,
        "gt" | "GT" | "Gt" | "gT" | ">" => Operator::MoreThan,
        "ge" | "GE" | "Ge" | "gE" | ">=" => Operator::MoreThanOrEqual,
        _ => panic!("Unknown operator: '{}'", op),
    };

    Expression::Comparison(ComparisonExpression {
        tag: tag_name.to_owned(),
        operator,
        value: value_name.to_owned(),
    })
}

fn white0(input: &str) -> IResult<&str, &str> {
    take_while(|c: char| c.is_whitespace())(input)
}

fn white1(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_whitespace())(input)
}

/// Match a keyword, either in lowercase or in uppercase, but not in mixed case
fn keyword(keyword: &str) -> impl Fn(&str) -> IResult<&str, &str>
where
{
    let k_lower = keyword.to_lowercase();
    let k_upper = keyword.to_uppercase();
    move |input| {
        let keyword_no_case = alt((tag(&k_lower as &str), tag(&k_upper as &str)));
        let peek_separator = peek(one_of(" ()"));
        terminated(keyword_no_case, peek_separator)(input)
    }
}

/// Parse a tag name: any escaped character or non-special character
/// (special meaning whitespace, parenthesis or comparison character)
fn tag_name(input: &str) -> IResult<&str, Expression> {
    let parser = escaped_transform(
        |input| take_till1(|c: char| SPECIAL_CHARS.contains(c) || c.is_whitespace())(input),
        '\\',
        |i: &str| take(1u8)(i),
    );

    // Check whether a parsed tag name is a keyword.
    // A simple equality check is not enough because we want to distinguish "or" from "\or". So we
    // also compare the length of the consumed string with the length of the keyword
    fn is_keyword(original_input_len: usize, remaining: &str, parsed: &str, keyword: &str) -> bool {
        (parsed.eq(&keyword.to_lowercase()) || parsed.eq(&keyword.to_uppercase()))
            && original_input_len == keyword.len() + remaining.len()
    }

    // Convert the string to an Expression, and make sure that reserved keywords are not used.
    // Note that "." and ".." cannot be used in the VFS.
    match parser(input) {
        Ok((s, tag)) => {
            for keyword in RESERVED_KEYWORDS {
                if is_keyword(input.len(), s, &tag, keyword) {
                    return Err(Err::Error((s, nom::error::ErrorKind::Tag)));
                }
            }
            Ok((s, make_tag(&tag)))
        }
        e => e.map(|(s, tag)| (s, make_tag(&tag))),
    }
}

fn comparison_expr(input: &str) -> IResult<&str, Expression> {
    // Textual operators are treated differently from symbol ones, as they require a space on the
    // left (and possibly right).
    // NOTE: when adding operators here, make sure to add them as well in make_comparison().
    let op_symbol = alt((
        tag("=="),
        tag("="),
        tag("!="),
        tag("<="),
        tag("<"),
        tag(">="),
        tag(">"),
    ));
    let op_text = alt((
        keyword("eq"),
        keyword("ne"),
        keyword("lt"),
        keyword("le"),
        keyword("gt"),
        keyword("ge"),
    ));
    let op_symbol_parser = preceded(space0, terminated(op_symbol, space0));
    let op_text_parser = preceded(space1, terminated(op_text, space1));
    let parser = tuple((tag_name, alt((op_symbol_parser, op_text_parser)), tag_name));

    parser(input).map(|(s, (tag, op, value))| {
        // We used tag_name() to parse the tag and value, so we have to unwrap the Expression.
        // Maybe not very elegant, but simple enough for now.
        if let (Expression::Tag(t), Expression::Tag(v)) = (tag, value) {
            return (s, make_comparison(&t.tag, op, &v.tag));
        }
        unreachable!("Bug: this code should be unreachable!");
    })
}

fn parens_expr(input: &str) -> IResult<&str, Expression> {
    delimited(char('('), full_expr, char(')'))(input)
}

/// Parse a NOT expression. Due to operator priority, its operand can only
/// be either a single tag name or a full expression between parentheses.
fn not_expr(input: &str) -> IResult<&str, Expression> {
    // Note that spaces are optional before parentheses
    let not_operand = alt((
        preceded(white0, parens_expr),
        preceded(white1, comparison_expr),
        preceded(white1, tag_name),
    ));
    let parser = preceded(keyword("not"), not_operand);

    parser(input).map(|(s, expr)| (s, make_not(expr)))
}

/// Parse an AND expression, with one or more operand(s).
/// The "and" keyword itself is optional.
/// Note that a NOT expression is a valid AND expression when parsing.
fn and_expr(input: &str) -> IResult<&str, Expression> {
    let and_operand = alt((parens_expr, not_expr, comparison_expr, tag_name));
    let optional_and = opt(tuple((keyword("and"), white0)));
    let and_keyword = tuple((white0, optional_and));
    let parser = tuple((&and_operand, many0(preceded(and_keyword, &and_operand))));

    parser(input).map(|(s, (left, right))| (s, fold(left, right, make_and)))
}

/// Parse an OR expression, with one or more operand(s).
/// Note that both NOT and AND expressions are valid OR expressions when parsing.
fn or_expr(input: &str) -> IResult<&str, Expression> {
    let or_keyword = delimited(white0, keyword("or"), white0);
    let parser = tuple((and_expr, many0(preceded(or_keyword, and_expr))));

    parser(input).map(|(s, (left, right))| (s, fold(left, right, make_or)))
}

fn full_expr(input: &str) -> IResult<&str, Expression> {
    delimited(white0, or_expr, white0)(input)
}

fn eof(input: &str) -> IResult<&str, ()> {
    not(take(1u8))(input)
}

/// Helper function to fold multiple Expression values (resulting from the parsing of an
/// associative operator such as "and" or "or") into a single Expression.
fn fold<F>(left: Expression, mut right: Vec<Expression>, merge: F) -> Expression
where
    F: Fn(Expression, Expression) -> Expression,
{
    // Insert the first element on the left (thus shifting everything)
    right.insert(0, left);

    while right.len() > 1 {
        let last = right.pop();
        let prev = right.pop();
        // This should always match, due to the while condition
        if let (Some(x), Some(y)) = (prev, last) {
            // "Merge" the 2 elements and add the result back to the vector
            right.push(merge(x, y));
        } else {
            unreachable!("Bug: this code should be unreachable!");
        }
    }

    if let Some(expr) = right.pop() {
        expr
    } else {
        unreachable!("Bug: this code should be unreachable!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_tag_name() {
        let assert_parse_tag = |input, expected| assert_parse(tag_name, input, &make_tag(expected));

        assert_parse_tag("aBc", "aBc");

        // Whitespace
        assert_parse_tag("a b c", "a");
        assert!(tag_name(" abc").is_err());
        assert_parse_tag(r"a\ b\ c", "a b c");
        assert_parse_tag("a\t", "a");
        assert_parse_tag("a\\\t", "a\t");
        assert_parse_tag("a\r", "a");
        assert_parse_tag("a\\\r", "a\r");
        assert_parse_tag("a\n", "a");
        assert_parse_tag("a\\\n", "a\n");

        // Special chars
        assert_parse_tag(r"abc(def)g", "abc");
        assert_parse_tag(r"abc\(def)g", "abc(def");
        assert_parse_tag(r"abc\(def\)g", "abc(def)g");
        assert_parse_tag(r"a<=2", "a");
        assert_parse_tag(r"a\<=2", "a<");
        assert_parse_tag(r"a\<\=2", "a<=2");

        // Keywords
        assert!(tag_name("not").is_err());
        assert!(tag_name("NOT  ").is_err());
        assert_parse_tag("NoT", "NoT");
        assert!(tag_name("and").is_err());
        assert!(tag_name("or").is_err());
        assert!(tag_name("eq").is_err());
        assert!(tag_name("ne").is_err());
        assert!(tag_name("lt").is_err());
        assert!(tag_name("le").is_err());
        assert!(tag_name("gt").is_err());
        assert!(tag_name("ge").is_err());
        assert_parse_tag(r"\or", "or");
        assert_parse_tag(r"o\r", "or");

        // Other
        assert!(tag_name("").is_err());
        assert!(tag_name(r"\").is_err());
        assert_parse_tag(r"\\\\", r"\\");
        assert_parse_tag(r"€ñอั喂", "€ñอั喂");
        assert_parse_tag("a b and c or not d e", "a");
    }

    #[test]
    fn can_parse_comparison_expr() {
        let assert_parse_comp =
            |input, t, op, v| assert_parse(comparison_expr, input, &make_comparison(t, op, v));

        // Equality symbol
        assert_parse_comp("a = 1", "a", "=", "1");
        assert_parse_comp("a   =  1", "a", "=", "1");
        assert_parse_comp("a=1", "a", "=", "1");
        assert_parse_comp("a==1", "a", "==", "1");
        assert_parse_comp(r"a=\=1", "a", "=", "=1");
        assert!(comparison_expr("a = =1").is_err());

        // Other symbols
        assert_parse_comp("a != 1", "a", "!=", "1");
        assert_parse_comp("a < 1", "a", "<", "1");
        assert_parse_comp("a <= 1", "a", "<=", "1");
        assert_parse_comp("a > 1", "a", ">", "1");
        assert_parse_comp("a >= 1", "a", ">=", "1");

        // Text
        assert_parse_comp("a eq 1", "a", "eq", "1");
        assert!(comparison_expr("a eq1").is_err());
        assert!(comparison_expr(r"a eq\1").is_err());
        assert!(comparison_expr(r"a\ eq1").is_err());
        assert_parse_comp("a ne 1", "a", "ne", "1");
        assert_parse_comp("a lt 1", "a", "lt", "1");
        assert_parse_comp("a le 1", "a", "le", "1");
        assert_parse_comp("a gt 1", "a", "gt", "1");
        assert_parse_comp("a ge 1", "a", "ge", "1");

        // Other
        assert!(comparison_expr("a << 1").is_err());
        assert!(comparison_expr("a <> 1").is_err());
        assert!(comparison_expr("a =! 1").is_err());
    }

    #[test]
    fn can_parse_not_expr() {
        let assert_parse_not =
            |input, expected| assert_parse(not_expr, input, &make_not(make_tag(expected)));

        // With keyword
        assert_parse_not("not foo", "foo");
        assert_parse_not("NOT   FoO ", "FoO");
        assert_parse_not("not(foo)", "foo");
        assert_parse_not("not ( (( foo)  ))", "foo");
        assert!(not_expr(r"\not foo").is_err());
        assert!(not_expr("not (a not) b").is_err());
        assert!(not_expr("not or").is_err());

        // With other expression
        assert_parse(
            not_expr,
            "not a > 1",
            &make_not(make_comparison("a", ">", "1")),
        );
        assert_parse(
            not_expr,
            "not (a > 1)",
            &make_not(make_comparison("a", ">", "1")),
        );
        assert_parse(
            not_expr,
            "not (a or b)",
            &make_not(make_or(make_tag("a"), make_tag("b"))),
        );
    }

    #[test]
    fn can_parse_and_expr() {
        let a_and_b_and_c = make_and(make_tag("a"), make_and(make_tag("b"), make_tag("c")));

        // a and (b and c)
        assert_parse(and_expr, "a and b and c", &a_and_b_and_c);
        assert_parse(and_expr, "a and b c", &a_and_b_and_c);
        assert_parse(and_expr, "a b and c", &a_and_b_and_c);
        assert_parse(and_expr, "a b c", &a_and_b_and_c);
        assert_parse(and_expr, "a (b and c)", &a_and_b_and_c);
        assert_parse(and_expr, "a and (b c)", &a_and_b_and_c);
        assert_parse(and_expr, "(( a )and(b ) c)", &a_and_b_and_c);
        assert_parse(and_expr, "(a)b(c)", &a_and_b_and_c);
        assert_parse(and_expr, "((a)((b))c)", &a_and_b_and_c);

        let assert_parse_and = |input, exp_left, exp_right| {
            assert_parse(and_expr, input, &make_and(exp_left, exp_right))
        };

        // Other tests
        assert_parse_and("a andb", make_tag("a"), make_tag("andb"));
        assert_parse_and(
            "a>1 b<2",
            make_comparison("a", ">", "1"),
            make_comparison("b", "<", "2"),
        );
        assert_parse_and(
            "1 + 2 = 3",
            make_tag("1"),
            make_and(make_tag("+"), make_comparison("2", "=", "3")),
        );
        assert_parse_and(
            "not a not b",
            make_not(make_tag("a")),
            make_not(make_tag("b")),
        );
        assert_parse_and(
            "( a or b ) and c",
            make_or(make_tag("a"), make_tag("b")),
            make_tag("c"),
        );
        // && is just a regular keyword
        assert_parse_and(
            "a && b",
            make_tag("a"),
            make_and(make_tag("&&"), make_tag("b")),
        );
    }

    #[test]
    fn can_parse_or_expr() {
        let assert_parse_or = |input, exp_left, exp_right| {
            assert_parse(or_expr, input, &make_or(exp_left, exp_right))
        };

        assert_parse_or("(( a )or(b))", make_tag("a"), make_tag("b"));
        assert_parse_or("not a or b", make_not(make_tag("a")), make_tag("b"));
        assert_parse_or(
            "a>b or(b!=1)",
            make_comparison("a", ">", "b"),
            make_comparison("b", "!=", "1"),
        );
        assert_parse_or(
            "a b or c and d",
            make_and(make_tag("a"), make_tag("b")),
            make_and(make_tag("c"), make_tag("d")),
        );
        assert_parse_or(
            "a>=a or(b b )",
            make_comparison("a", ">=", "a"),
            make_and(make_tag("b"), make_tag("b")),
        );

        assert_parse(or_expr, "a and b", &make_and(make_tag("a"), make_tag("b")));
        assert_parse(or_expr, "a orb", &make_and(make_tag("a"), make_tag("orb")));
    }

    #[test]
    fn can_parse_full_expr() {
        assert_parse(full_expr, r" \<foo/\> ", &make_tag("<foo/>"));
        assert_parse(
            full_expr,
            r"   (&<#\!)    ",
            &make_comparison("&", "<", "#!"),
        );
        assert_parse(full_expr, " a or a", &make_or(make_tag("a"), make_tag("a")));

        // Garbage at EOS: OK for full_expr() but not for parse_expr()
        assert_parse(full_expr, " a = ", &make_tag("a"));
        assert!(parse_expr(" a = ").is_err());
        assert_parse(full_expr, "a and ()", &make_tag("a"));
        assert!(parse_expr("a and ()").is_err());

        // Other errors
        assert!(full_expr("").is_err());
        assert!(full_expr("()").is_err());
    }

    #[test]
    fn test_parse_whitespace() {
        assert!(parse_whitespace("").is_ok());
        assert!(parse_whitespace(" \n\t\r    ").is_ok());
        assert!(parse_whitespace("     r").is_err());
    }

    // Helper function to remove some boilerplate
    fn assert_parse<F>(parsinc_func: F, to_parse: &str, expected_expr: &Expression)
    where
        F: Fn(&str) -> IResult<&str, Expression>,
    {
        match parsinc_func(to_parse) {
            Ok((_, expr)) => assert_eq!(&expr, expected_expr, "\nexpr: [{}]", to_parse),
            Err(e) => panic!(format!("Parsing failed for [{}]: {:?}", to_parse, &e)),
        }
    }
}
