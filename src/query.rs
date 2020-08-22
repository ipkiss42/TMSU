mod parser;

use crate::errors::*;

#[derive(Debug, PartialEq)]
pub(crate) enum Expression {
    Tag(TagExpression),
    Not(NotExpression),
    And(AndExpression),
    Or(OrExpression),
    Comparison(ComparisonExpression),
}

#[derive(Debug, PartialEq)]
pub(crate) struct TagExpression {
    pub tag: String,
}

#[derive(Debug, PartialEq)]
pub(crate) struct NotExpression {
    pub operand: Box<Expression>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct AndExpression {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct OrExpression {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ComparisonExpression {
    pub tag: String,
    pub operator: Operator,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub enum Operator {
    Different,
    Equal,
    LessThan,
    LessThanOrEqual,
    MoreThan,
    MoreThanOrEqual,
}

impl Expression {
    pub(crate) fn parse(query: &str) -> Result<Option<Self>> {
        // Whitespace only -> None
        if parser::parse_whitespace(query).is_ok() {
            return Ok(None);
        }

        // Anything else -> Some(parsed_expression)
        let (_, expr) = parser::parse_expr(query)
            .map_err(|_| ErrorKind::QueryParsingError(query.to_owned()))?;
        Ok(Some(expr))
    }

    pub(crate) fn tag_names(&self) -> Vec<&str> {
        let mut names = vec![];
        self.tag_names_rec(&mut names);
        names
    }

    fn tag_names_rec<'a>(&'a self, names: &mut Vec<&'a str>) {
        match self {
            Expression::Tag(tag_expr) => names.push(&tag_expr.tag),
            Expression::Not(not_expr) => not_expr.operand.tag_names_rec(names),
            Expression::And(and_expr) => {
                and_expr.left.tag_names_rec(names);
                and_expr.right.tag_names_rec(names);
            }
            Expression::Or(or_expr) => {
                or_expr.left.tag_names_rec(names);
                or_expr.right.tag_names_rec(names);
            }
            Expression::Comparison(comp_expr) => names.push(&comp_expr.tag),
        }
    }

    pub(crate) fn exact_value_names(&self) -> Vec<&str> {
        let mut names = vec![];
        self.exact_value_names_rec(&mut names);
        names
    }

    fn exact_value_names_rec<'a>(&'a self, names: &mut Vec<&'a str>) {
        match self {
            Expression::Tag(_) => (),
            Expression::Not(not_expr) => not_expr.operand.exact_value_names_rec(names),
            Expression::And(and_expr) => {
                and_expr.left.exact_value_names_rec(names);
                and_expr.right.exact_value_names_rec(names);
            }
            Expression::Or(or_expr) => {
                or_expr.left.exact_value_names_rec(names);
                or_expr.right.exact_value_names_rec(names);
            }
            Expression::Comparison(comp_expr) => match comp_expr.operator {
                Operator::Equal | Operator::Different => names.push(&comp_expr.value),
                Operator::LessThan
                | Operator::LessThanOrEqual
                | Operator::MoreThan
                | Operator::MoreThanOrEqual => (),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expr_parse() -> Result<()> {
        // Empty expressions
        assert_eq!(Expression::parse("")?, None);
        assert_eq!(Expression::parse("  \t \r\n ")?, None);

        // Basic test only: actual parsing tests are done directly for the parsing functions
        let expr = Expression::parse("  hello  ")?;
        assert_eq!(
            expr,
            Some(Expression::Tag(TagExpression {
                tag: "hello".to_owned()
            }))
        );

        Ok(())
    }

    #[test]
    fn expr_tag_names() -> Result<()> {
        let expr =
            Expression::parse("not (not b) (a) or c = 2 or d == 3 or e != 4 or f > 5")?.unwrap();

        let mut actual_names = expr.tag_names();
        actual_names.sort();

        assert_eq!(actual_names, vec!["a", "b", "c", "d", "e", "f"]);
        Ok(())
    }

    #[test]
    fn expr_exact_value_names() -> Result<()> {
        let expr =
            Expression::parse("not (not b) (a) or c = 2 or d == 3 or e != 4 or f > 5")?.unwrap();

        let mut actual_names = expr.exact_value_names();
        actual_names.sort();

        assert_eq!(actual_names, vec!["2", "3", "4"]);
        Ok(())
    }
}
