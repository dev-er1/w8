<div align="center">

# **W8**

![Rust](https://img.shields.io/badge/MSRV-1.96.0-blue?style=flat-square&logo=rust&logoColor=white)
![CI](https://github.com/dev-er1/w8/actions/workflows/ci.yml/badge.svg)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue?style=flat-square)](LICENSE)

**[Контрибьютинг](CONTRIBUTING.ru.md) | [Архитектура](docs/Architecture/Architecture.ru.md) | [NB формат](docs/File-Format/File-Format.ru.md) | [CoC](CODE_OF_CONDUCT.ru.md) | [Лицензия](LICENSE)**

</div>

**W8** — регистровая виртуальная машина с 64-битными регистрами и JIT-компилятором (пока что, только для x64).

## Превью
Дòнат:
![](gifs/donut.gif)

## Установка
Есть несколько способов получить W8 — выберите подходящий:

### 1. Готовый бинарник из GitHub Releases
Скачайте архив для вашей платформы со страницы [последнего релиза](https://github.com/dev-er1/w8/releases/latest) и добавьте
путь к бинарнику `w8c` или `w8c.exe` в `PATH`.

### 2. Сборка из исходников
Требуется [Rust](https://www.rust-lang.org/) версии не ниже **1.96.0**:

```sh
git clone https://github.com/dev-er1/w8.git
cd wdt/wdc
cargo build --release
```
Бинарник появится в `target/release`. Чтобы использовать его из любого места, добавьте этот путь в `PATH` или скопируйте бинарник в удобный каталог.
