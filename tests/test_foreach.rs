use ponos_rs::ponos::Ponos;

#[test]
fn test_foreach_simple() {
    let source = r#"
        функ тест()
            пер arr = [1, 2, 3];
            пер сумма = 0;

            для каждого x из arr
                сумма = сумма + x;
            конец

            возврат сумма;
        конец

        пер результат = тест();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_foreach_with_index() {
    let source = r#"
        функ тест()
            пер arr = [10, 20, 30];
            пер взвешенная = 0;

            для каждого значение, индекс из arr
                взвешенная = взвешенная + (значение * индекс);
            конец

            возврат взвешенная;
        конец

        пер результат = тест();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_foreach_empty_array() {
    let source = r#"
        функ тест()
            пер arr = [];
            пер счетчик = 0;

            для каждого x из arr
                счетчик = счетчик + 1;
            конец

            возврат счетчик;
        конец

        пер результат = тест();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_foreach_nested() {
    let source = r#"
        функ тест()
            пер внешний = [1, 2];
            пер внутренний = [10, 20];
            пер сумма = 0;

            для каждого x из внешний
                для каждого y из внутренний
                    сумма = сумма + (x * y);
                конец
            конец

            возврат сумма;
        конец

        пер результат = тест();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_foreach_with_module() {
    let source = r#"
        использовать "стд/ввод_вывод" как ио;

        функ тест()
            пер arr = [1, 2];

            для каждого x из arr
                ио.вывести(x);
            конец
        конец

        тест();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}
