use ponos_rs::ponos::Ponos;

// Тесты для логических операторов И и ИЛИ с коротким замыканием

#[test]
fn test_and_operator_both_true() {
    let source = r#"
        пер x = истина и истина;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_and_operator_first_false() {
    let source = r#"
        пер x = ложь и истина;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_and_operator_second_false() {
    let source = r#"
        пер x = истина и ложь;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_and_operator_both_false() {
    let source = r#"
        пер x = ложь и ложь;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_or_operator_both_true() {
    let source = r#"
        пер x = истина или истина;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_or_operator_first_true() {
    let source = r#"
        пер x = истина или ложь;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_or_operator_second_true() {
    let source = r#"
        пер x = ложь или истина;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_or_operator_both_false() {
    let source = r#"
        пер x = ложь или ложь;
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_and_short_circuit() {
    // Короткое замыкание: если первый операнд false, второй не вычисляется
    let source = r#"
        пер a = ложь;
        пер b = 10;
        пер x = a и (b > 5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_or_short_circuit() {
    // Короткое замыкание: если первый операнд true, второй не вычисляется
    let source = r#"
        пер a = истина;
        пер b = 10;
        пер x = a или (b > 5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_combined_logical_operators() {
    let source = r#"
        пер x = истина и (ложь или истина);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_logical_operators_with_comparison() {
    let source = r#"
        пер a = 5;
        пер b = 10;
        пер c = 3;
        пер x = (a < b) и (b > c);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}
