use ponos_rs::ponos::Ponos;

// Тесты для управляющих конструкций

// === Тесты для if/else ===

#[test]
fn test_if_true() {
    let source = r#"
        пер x = 0;
        если истина
            x = 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_false() {
    let source = r#"
        пер x = 0;
        если ложь
            x = 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_with_comparison() {
    let source = r#"
        пер a = 10;
        пер b = 5;
        пер результат = 0;
        если a > b
            результат = 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_else_true_branch() {
    let source = r#"
        пер x = 0;
        если истина
            x = 1;
        иначе
            x = 2;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_else_false_branch() {
    let source = r#"
        пер x = 0;
        если ложь
            x = 1;
        иначе
            x = 2;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_multiple_statements() {
    let source = r#"
        пер x = 0;
        пер y = 0;
        если истина
            x = 1;
            y = 2;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_nested_if() {
    let source = r#"
        пер x = 0;
        если истина
            если истина
                x = 1;
            конец
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_with_logical_operators() {
    let source = r#"
        пер a = истина;
        пер b = истина;
        пер x = 0;
        если a и b
            x = 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// === Тесты для while ===

#[test]
fn test_while_zero_iterations() {
    let source = r#"
        пер x = 0;
        пока ложь
            x = x + 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_while_with_counter() {
    let source = r#"
        пер i = 0;
        пока i < 5
            i = i + 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_while_with_multiple_statements() {
    let source = r#"
        пер i = 0;
        пер сумма = 0;
        пока i < 3
            сумма = сумма + i;
            i = i + 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_nested_while() {
    let source = r#"
        пер i = 0;
        пер j = 0;
        пока i < 2
            j = 0;
            пока j < 2
                j = j + 1;
            конец
            i = i + 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_while_with_if() {
    let source = r#"
        пер i = 0;
        пер чётные = 0;
        пока i < 10
            если i == 0
                чётные = чётные + 1;
            конец
            если i == 2
                чётные = чётные + 1;
            конец
            если i == 4
                чётные = чётные + 1;
            конец
            i = i + 1;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_if_else_with_arithmetic() {
    let source = r#"
        пер x = 10;
        пер результат = 0;
        если x > 5
            результат = x * 2;
        иначе
            результат = x / 2;
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}
