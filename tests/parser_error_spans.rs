use ponos_rs::ponos::Ponos;

fn line_col_at(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (idx, ch) in src.char_indices() {
        if idx == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[test]
fn parser_points_to_real_error_location() {
    let source = r#"функ инициализировать_пакет()
  пер содержимое_манифеста = {
    "наименование": "тест",
    "зависимости": {
      "тестовая-зависимость": "1.1.1"
    }
    "точка_входа": "тест.pns"
  };
конец"#;

    let mut ponos = Ponos::new();
    let err = ponos
        .parse_only(source.to_string())
        .expect_err("должна быть ошибка синтаксиса");

    eprintln!("{}", err.format(source, "<test>"));

    let expected_offset = source
        .find("\"точка_входа\"")
        .expect("пример должен содержать ключ 'точка_входа'");
    let (expected_line, expected_col) = line_col_at(source, expected_offset);

    let (start, _) = err.span.to_location(source);
    assert_eq!(
        (start.line, start.column),
        (expected_line, expected_col),
        "span должен указывать на начало проблемного ключа"
    );
}

#[test]
fn parser_points_to_error_in_full_program() {
    let source = r#"использовать "стд/ввод_вывод" как ио;
использовать "стд/сеть";
использовать "стд/json";
использовать "стд/фс";
использовать "стд/строки";
использовать "стд/система" как сис;

функ старт() 
  
  пер аргументы = сис.аргументы();
  пер количество_аргументов = длина(аргументы);
  пер индекс = 0;
  пока индекс < количество_аргументов 
    индекс = индекс + 1;
  конец
  
  пер команда = распарсить(аргументы);
  выполнить_команду(команда);

конец

класс Команда
  конструктор(имя, аргументы, флаги)
    это.имя = имя;
    это.аргументы = аргументы;
    это.флаги = флаги;
  конец
конец

// аргументы - массив из строк 
функ распарсить(аргументы)
  если длина(аргументы) == 0
    исключение "передан пустой массив аргументов";
  конец

  если длина(аргументов) < 3
    исключение "недостаточно аргументов";
  конец

  пер имя = нормализовать_имя_комадны(аргументов[2]);
  пер аргументы = [];
  пер флаги = {};

  пер индекс = 3;
  пока индекс < длина(аргументов)
    пер арг = аргументы[индекс];

    если строки.начинается_с(арг, "--")
      пер имя_флага = арг[2:];
      флаги[имя_флага] = истина;
    иначе если строки.начинается_с(арг, "-")
      пер имя_флага = арг[1:];
      флаги[имя_флага] = истина;
    иначе
      аргументы.добавить(арг);
    конец

    индекс = индекс + 1;
  конец

  возврат Команда(имя, аргументы, флаги);
конец

функ нормализовать_имя_комадны(имя)
  пер имена = {
      "init": "инициализировать"
  };

  возврат имена[имя] или имя;

конец

функ выполнить_команду(команда)
  если команда.имя == "инициализировать" 
    инициализировать_пакет();
  конец
конец

функ инициализировать_пакет()
  
  пер содержимое_манифеста = {
    "наименование": "тест",
    "версия": "1.0.0",
    "зависимости": {
      "тестовая-зависимость": "1.1.1"
    }
    "точка_входа": "тест.pns"
  };

  пер сериализованное_содержание_манифеста = json.сериализовать(содержимое_манифеста);
  фс.писать("пакет.json", сериализованное_содержание_манифеста);

конец

старт();
"#;

    let mut ponos = Ponos::new();
    let err = ponos
        .parse_only(source.to_string())
        .expect_err("должна быть ошибка синтаксиса");

    eprintln!("{}", err.format(source, "<full>"));

    let expected_offset = source
        .find("\"точка_входа\"")
        .expect("пример должен содержать ключ 'точка_входа'");
    let (expected_line, expected_col) = line_col_at(source, expected_offset);
    let (start, _) = err.span.to_location(source);
    assert_eq!(
        (start.line, start.column),
        (expected_line, expected_col),
        "span должен указывать на ключ без запятой в полном примере"
    );
}

#[test]
fn parser_reports_missing_semicolon_before_end() {
    let source = r#"функ f()
  возврат 1
конец
"#;

    let mut ponos = Ponos::new();
    let err = ponos
        .parse_only(source.to_string())
        .expect_err("должна быть ошибка из-за отсутствующей ;");

    let expected_offset = source
        .find("конец")
        .expect("пример должен содержать конец");
    let (expected_line, expected_col) = line_col_at(source, expected_offset);
    let (start, _) = err.span.to_location(source);
    assert_eq!(
        (start.line, start.column),
        (expected_line, expected_col),
        "span должен указывать на начало 'конец', где парсер ожидает ';'"
    );
}
