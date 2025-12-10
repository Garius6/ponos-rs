use ponos_rs::ponos::Ponos;

#[test]
fn test_minimal_class() {
    let source = r#"
        класс Точка
        конец
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
    // Тест просто проверяет, что нет паники
}

#[test]
fn test_class_with_constructor() {
    let source = r#"
        класс Точка
            конструктор()
            конец
        конец

        пер p = Точка();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_class_with_field_and_constructor() {
    let source = r#"
        класс Точка
            x: число

            конструктор(val: число)
                это.x = val;
            конец
        конец

        пер p = Точка(42);
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_class_with_method() {
    let source = r#"
        класс Точка
            x: число

            конструктор(val: число)
                это.x = val;
            конец

            функ получить_x(): число
                возврат это.x;
            конец
        конец

        пер p = Точка(42);
        пер результат = p.получить_x();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_class_method_call() {
    let source = r#"
        класс Калькулятор
            значение: число

            конструктор(val: число)
                это.значение = val;
            конец

            функ удвоить(): число
                возврат это.значение;
            конец
        конец

        пер calc = Калькулятор(21);
        пер result = calc.удвоить();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

// Тесты наследования

#[test]
fn test_simple_inheritance() {
    let source = r#"
        класс Животное
            конструктор()
            конец

            функ говорить(): строка
                возврат "Животное";
            конец
        конец

        класс Собака наследует Животное
            конструктор()
            конец
        конец

        пер собака = Собака();
        пер речь = собака.говорить();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_inheritance_with_override() {
    let source = r#"
        класс Животное
            конструктор()
            конец

            функ говорить(): строка
                возврат "Животное говорит";
            конец
        конец

        класс Собака наследует Животное
            конструктор()
            конец

            функ говорить(): строка
                возврат "Гав-гав";
            конец
        конец

        пер собака = Собака();
        пер речь = собака.говорить();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_inheritance_with_own_methods() {
    let source = r#"
        класс Животное
            конструктор()
            конец

            функ говорить(): строка
                возврат "Животное";
            конец
        конец

        класс Собака наследует Животное
            конструктор()
            конец

            функ лаять(): строка
                возврат "Гав";
            конец
        конец

        пер собака = Собака();
        пер речь = собака.говорить();
        пер лай = собака.лаять();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}

#[test]
fn test_inheritance_with_fields() {
    let source = r#"
        класс Животное
            имя: строка

            конструктор(имя_val: строка)
                это.имя = имя_val;
            конец

            функ получить_имя(): строка
                возврат это.имя;
            конец
        конец

        класс Собака наследует Животное
            конструктор(имя_val: строка)
                это.имя = имя_val;
            конец
        конец

        пер собака = Собака("Бобик");
        пер имя = собака.получить_имя();
    "#;

    let mut ponos = Ponos::new();
    ponos.run_source(source.to_string());
}
