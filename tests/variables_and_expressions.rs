use ponos_rs::ponos::Ponos;

// Тесты для переменных и выражений

// === Тесты переменных ===

#[test]
fn test_local_variable_number() {
    let source = r#"
        пер x = 42;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_local_variable_string() {
    let source = r#"
        пер имя = "Понос";
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_local_variable_boolean() {
    let source = r#"
        пер флаг = истина;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_multiple_local_variables() {
    let source = r#"
        пер a = 1;
        пер b = 2;
        пер c = 3;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_variable_reassignment() {
    let source = r#"
        пер x = 10;
        x = 20;
        x = 30;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_variable_with_expression() {
    let source = r#"
        пер x = 5 + 3;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// === Тесты арифметических операторов ===

#[test]
fn test_addition() {
    let source = r#"
        пер результат = 5 + 3;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_subtraction() {
    let source = r#"
        пер результат = 10 - 4;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_multiplication() {
    let source = r#"
        пер результат = 6 * 7;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_division() {
    let source = r#"
        пер результат = 20 / 4;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_arithmetic_precedence() {
    let source = r#"
        пер результат = 2 + 3 * 4;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_arithmetic_with_parentheses() {
    let source = r#"
        пер результат = (2 + 3) * 4;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_unary_negation() {
    let source = r#"
        пер x = -5;
        пер y = -x;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// === Тесты операторов сравнения ===

#[test]
fn test_less_than() {
    let source = r#"
        пер результат = 5 < 10;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_less_than_or_equal() {
    let source = r#"
        пер результат = 5 <= 5;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_greater_than() {
    let source = r#"
        пер результат = 10 > 5;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_greater_than_or_equal() {
    let source = r#"
        пер результат = 10 >= 10;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_equality() {
    let source = r#"
        пер результат = 5 == 5;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_inequality() {
    let source = r#"
        пер результат = 5 != 3;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// === Тесты логического отрицания ===

#[test]
fn test_logical_not_true() {
    let source = r#"
        пер результат = !истина;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_logical_not_false() {
    let source = r#"
        пер результат = !ложь;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// === Тесты комбинаций операторов ===

#[test]
fn test_complex_expression() {
    let source = r#"
        пер a = 5;
        пер b = 10;
        пер c = (a + b) * 3 - 2;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_comparison_with_arithmetic() {
    let source = r#"
        пер x = (5 + 3) > (10 - 2);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_logical_with_comparison() {
    let source = r#"
        пер результат = (5 > 3) и (10 < 20);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}
