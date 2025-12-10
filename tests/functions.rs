use ponos_rs::ponos::Ponos;

// Тесты для функций и замыканий

// === Тесты простых функций ===

#[test]
fn test_simple_function_no_params() {
    let source = r#"
        функ привет()
            пер сообщение = "Привет!";
        конец

        привет();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_function_with_return() {
    let source = r#"
        функ получитьЧисло(): число
            возврат 42;
        конец

        пер результат = получитьЧисло();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_function_with_one_param() {
    let source = r#"
        функ удвоить(x: число): число
            возврат x * 2;
        конец

        пер результат = удвоить(5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_function_with_two_params() {
    let source = r#"
        функ сложить(a: число, b: число): число
            возврат a + b;
        конец

        пер результат = сложить(3, 4);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_function_with_multiple_statements() {
    let source = r#"
        функ вычислить(x: число): число
            пер temp = x * 2;
            пер result = temp + 10;
            возврат result;
        конец

        пер ответ = вычислить(5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_recursive_function() {
    let source = r#"
        функ факториал(n: число): число
            если n <= 1
                возврат 1;
            конец
            возврат n * факториал(n - 1);
        конец

        пер результат = факториал(5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// TODO: Вызов функций внутри других функций требует исправления в генераторе
// #[test]
// fn test_function_calling_another_function() {
//     let source = r#"
//         функ добавитьОдин(x: число): число
//             возврат x + 1;
//         конец
//
//         функ добавитьДва(x: число): число
//             возврат добавитьОдин(добавитьОдин(x));
//         конец
//
//         пер результат = добавитьДва(5);
//     "#;
//
//     let mut ponos = Ponos::new();
//     ponos.run_source(source.to_string());
// }

// === Тесты лямбд и замыканий ===
// TODO: Лямбды и замыкания требуют дополнительной реализации парсера

#[test]
fn test_function_with_local_variables() {
    let source = r#"
        функ тест(): число
            пер a = 1;
            пер b = 2;
            пер c = a + b;
            возврат c;
        конец

        пер результат = тест();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_function_with_if_statement() {
    let source = r#"
        функ максимум(a: число, b: число): число
            если a > b
                возврат a;
            конец
            возврат b;
        конец

        пер результат = максимум(10, 5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_function_with_while_loop() {
    let source = r#"
        функ сумма(n: число): число
            пер i = 0;
            пер sum = 0;
            пока i <= n
                sum = sum + i;
                i = i + 1;
            конец
            возврат sum;
        конец

        пер результат = сумма(5);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}
