use crate::ponos::parser::combinator::{Input, PResult};
use winnow::prelude::*;

/// Тип аннотации (временная заглушка, будет расширена)
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Named(String),
    // TODO: добавить другие варианты типов
}

/// Парсит аннотацию типа
pub fn parse_type_annotation<'a>(input: &mut Input<'a>) -> PResult<'a, TypeAnnotation> {
    // TODO: : type_expression
    todo!("parse_type_annotation not yet implemented")
}

/// Парсит выражение типа
pub fn parse_type_expression<'a>(input: &mut Input<'a>) -> PResult<'a, TypeAnnotation> {
    // TODO: Будет реализовано в Этапе 5
    todo!("parse_type_expression not yet implemented")
}
