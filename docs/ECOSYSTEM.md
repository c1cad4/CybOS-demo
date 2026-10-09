# CybOS-demo в экосистеме cybOS

Нативный рабочий стол и тестовый стенд, собирающий самостоятельные компоненты.

[Общая карта](https://github.com/c1cad4/cybOS) · [Тестовый стенд](https://github.com/c1cad4/CybOS-demo) · [Каталог компонентов](https://github.com/c1cad4/cybOS/blob/main/ecosystem.json)

## Ответственность

Категория: `system`. Тип компонента: `application`.

Соседние зависимости: `robotcyb-core`, `cybnet`, `cybguard`, `cybbee`, `cybchain`, `cybweb`, `CybBrowser`, `cybdex`, `soul`, `immunocybchain`.

## Проверка

Из корня репозитория:

```bash
cargo test --locked -j 4
```
