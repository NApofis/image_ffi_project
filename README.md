# image_ffi_project

Проект для обработки PNG-изображений через динамически подключаемые плагины.

Воркспейс состоит из общей библиотеки `image_processor`, CLI-приложения для запуска обработки и двух плагинов:

- `mirror_plugin` — зеркалирование изображения;
- `blur_plugin` — размытие изображения;
- `image_processor` — общая библиотека с типами, helper-функциями, FFI-контрактом и CLI-бинарём.

## Структура проекта

```text
image_ffi_project/
├── Cargo.toml                     # workspace
├── image_processor/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # общая библиотека для плагинов и CLI
│   │   ├── cli.rs                 # вспомогательные функции для CLI
│   │   ├── error.rs               # ошибки CLI
│   │   ├── plugin_loader.rs       # загрузка динамических библиотек
│   │   └── bin/
│   │       └── image_processor.rs # CLI-бинарь
│   └── example_files/
│       ├── image_1.png
│       ├── image_2.png
│       ├── image_3.png
│       └── params.json
└── plugins/
    ├── blur_plugin/
    │   ├── Cargo.toml
    │   └── src/lib.rs
    └── mirror_plugin/
        ├── Cargo.toml
        └── src/lib.rs
```

---

## Фича `cli`

В пакете `image_processor` бинарник собирается **только** с фичей `cli`.

Это сделано специально:

- без `cli` пакет `image_processor` остаётся лёгкой библиотекой для плагинов;
- в плагины не подтягиваются лишние зависимости вроде `clap`, `image`, `libloading`, `anyhow`;
- CLI-часть подключается только тогда, когда действительно нужен исполняемый файл.

---

## Сборка проекта

### 1. Сборка всего workspace

Из корня проекта:

```bash
cargo build
```

Что будет собрано:

- библиотека `image_processor`;
- плагин `mirror_plugin`;
- плагин `blur_plugin`.

На Linux плагины будут собраны как `.so`, на Windows — как `.dll`.

### 2. Сборка CLI

CLI нужно собирать отдельно с фичей `cli`:

```bash
cargo build -p image_processor --features cli --bin image_processor
```

## Что именно собирается

### Библиотека `image_processor`

Содержит:

- общие типы и константы;
- парсинг JSON-параметров;
- валидацию FFI-параметров;
- сигнатуру логгера, который передаётся в плагин;
- FFI-контракт для плагинов.

### CLI `image_processor`

CLI умеет:

- валидировать входные параметры;
- читать PNG;
- читать JSON;
- загружать плагин через `libloading`;
- вызывать `process_image`;
- сохранять итоговый файл.

### Плагины

Оба плагина собираются как `cdylib`:

- `plugins/mirror_plugin`;
- `plugins/blur_plugin`.

Это динамические библиотеки, которые загружает CLI во время выполнения.

---

## Использование CLI

### Синтаксис

```bash
image_processor \
  --input <PATH_TO_INPUT_PNG> \
  --output <PATH_TO_OUTPUT_DIR> \
  --plugin <PLUGIN_NAME_WITHOUT_EXTENSION> \
  --params <PATH_TO_JSON> \
  [--plugin-path <PATH_TO_PLUGIN_DIR>]
```

### Аргументы

- `--input` — путь к исходному PNG-файлу;
- `--output` — путь к **директории**, куда будет сохранён результат;
- `--plugin` — имя плагина **без** `.so` / `.dll` и без `lib`;
- `--params` — путь к JSON-файлу с параметрами плагина;
- `--plugin-path` — каталог, где лежит динамическая библиотека плагина. По умолчанию используется путь `../target/debug`

### Важно

`--output` должен указывать именно на **существующую директорию**, а не на имя итогового файла.

Итоговый файл сохраняется автоматически с префиксом `new_`:

```text
new_<имя_входного_файла>
```

Например:

- входной файл: `image_1.png`
- результат: `new_image_1.png`

---

## Примеры запуска

Ниже примеры для запуска из **корня workspace**.

### Вариант 1. Запуск через `cargo run`

#### Mirror plugin

```bash
cargo run -p image_processor --features cli --bin image_processor -- \
  --input image_processor/example_files/image_1.png \
  --output image_processor/example_files \
  --plugin mirror_plugin \
  --params image_processor/example_files/params.json \
  --plugin-path target/debug
```

#### Blur plugin

```bash
cargo run -p image_processor --features cli --bin image_processor -- \
  --input image_processor/example_files/image_1.png \
  --output image_processor/example_files \
  --plugin blur_plugin \
  --params image_processor/example_files/params.json \
  --plugin-path target/debug
```

## Формат JSON-параметров

### `mirror_plugin`

Плагин поддерживает параметры:

```json
{
  "horizontal": true,
  "vertical": false
}
```

Ключи:

- horizontal - отзиркалить по горизонтали
- vertical - отзиркалить по вертикали

Нужно указать хотя бы один из них.

### `blur_plugin`

Плагин ожидает:

```json
{
  "radius": 3,
  "iterations": 2
}
```

Параметры:

- `radius` — радиус размытия, положительное число;
- `iterations` — количество проходов размытия, положительное число.

---

## Расширение проекта

Чтобы добавить новый плагин, нужно:

1. создать новый crate в `plugins/...`;
2. собрать его как `cdylib`;
3. экспортировать функцию `process_image` с нужной сигнатурой;
4. использовать библиотеку `image_processor` для общих типов, ошибок и helper-функций.

Ожидаемая сигнатура экспортируемой функции:

```rust
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
    log_fn: LogFn,
)
```