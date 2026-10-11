# Карта репозиториев и правила объединения

Проверено 11 октября 2026: GitHub API показывает 39 публичных репозиториев. Назначение сверено с README и деревьями файлов. Автоматические проверки относятся только к явно перечисленным ниже проектам; наличие кода не означает готовый продукт.

## Канонические точки разработки

- **CybOS-demo** — нативное приложение: UI, SQLite, связь модулей и пользовательские сценарии. Продолжаем разработку рабочего стола здесь.
- **cybOS** — каталог, lock-файлы, общий установщик и карта экосистемы.
- **CybCore** — опциональный локальный Python API для агентов и памяти. Его не нужно встраивать как обязательный сервер рабочего стола.
- **cicadafarm-game** — исходники Swarm; копия на сайте cicadafarm нужна для совместимости старых ссылок. Изменения игры делаются в исходном проекте и затем синхронизируются.

## Что объединяем

Объединяем рабочие сценарии и контракты через существующие интеграторы. Десять Rust-библиотек уже подключены в CybOS-demo; четыре Python-компонента — в CybCore. Перенос файлов в один репозиторий сейчас создаст два владельца API и сломает закреплённые зависимости.

Тринадцать проектов с одними README/LICENSE пока не требуют отдельных реализаций. Их задачи сгруппированы в общем плане:

- CybMind — интеллектуальная подсистема над CybAgents + CybSwarm + CybRegistry, подключаемая через CybCore.
- CybBrain — обучение и граф над CybMemory / soul; хранилище и формат знаний согласуются до новой реализации.
- CybShield — общий сценарий безопасности; CybIdentity и CybTrust — контракты ключей и доверия. Существующие cybguard и immunocybchain сохраняют конкретные функции.
- CybRobot и CybWorld — адаптеры устройств и цифровой двойник; сначала один наблюдаемый датчик через robotcyb-core + cybbee.
- CybGrowth, CybMentor и CybMissions — один сценарий обучения с заданиями, результатами и источниками.
- CybWallet и CybExplorer — отдельная будущая вертикаль чтения/подписи; подпись не добавляется в read-only cybdex.
- CybLab — экспериментальные возможности только после определения владельца и контракта.

Исходные репозитории и история сохранены. Физическое объединение/архивация не выполнялись. План не означает, что эти 13 компонентов реализованы.

## Описания всех публичных проектов

### [cicadafarm](https://github.com/c1cad4/cicadafarm)

Публичный цифровой двойник фермы: навигация, знания, демо наблюдений и вход в экосистему.

Статус: website. Категория: physical.
Граница: Показания фермы и ответы чата демонстрационные; сайт не управляет оборудованием.
Проверка: `node --test`.

### [cicadafarm-game](https://github.com/c1cad4/cicadafarm-game)

Канонический исходный код игры CicadaFarm Swarm: одиночный режим и мультиплеер.

Статус: application. Категория: experience.
Граница: Мультиплеер и аккаунты требуют Cloudflare Durable Objects; статический просмотр проверяет браузерную часть.
Проверка: `node --test`.

### [CybAgents](https://github.com/c1cad4/CybAgents)

сами агенты: их роли, инструменты, память в рамках задач, настройки, жизненный цикл и протоколы взаимодействия..

Статус: library. Категория: intelligence.
Проверка: `python3 -m unittest discover -s tests`.

### [cybbee](https://github.com/c1cad4/cybbee)

Наблюдения фермы и пасеки с источником и временем.

Статус: library. Категория: physical.
Проверка: `cargo test --locked -j 4`.

### [CybBrain](https://github.com/c1cad4/CybBrain)

Долговременная память, граф знаний, обучение и накопление опыта..

Статус: Архитектурный план: только README и LICENSE. Категория: intelligence.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybBrowser](https://github.com/c1cad4/CybBrowser)

Ограниченный загрузчик документов: HTTP, IPFS/IPNS и Arweave.

Статус: library. Категория: network.
Подключает: cybweb.
Проверка: `cargo test --locked -j 4`.

### [cybchain](https://github.com/c1cad4/cybchain)

Валидация значений внешних оракулов; консенсус остаётся в go-cyber.

Статус: library. Категория: markets.
Проверка: `cargo test --locked -j 4`.

### [cybchat](https://github.com/c1cad4/cybchat)

Самостоятельное приложение общения для iOS/macOS.

Статус: platform-limited. Категория: network.
Граница: Требуется macOS/iOS SDK и Xcode.

### [CybCore](https://github.com/c1cad4/CybCore)

Серверная инфраструктура, API, узлы сети и координация сервисов..

Статус: application. Категория: system.
Подключает: CybAgents, CybRegistry, CybMemory, CybSwarm.
Проверка: `python3 -m unittest discover -s tests`.

### [cybdex](https://github.com/c1cad4/cybdex)

Данные рынков Solana: поиск пар и свечи, без исполнения обменов.

Статус: library. Категория: markets.
Подключает: robotcyb-core.
Проверка: `cargo test --locked -j 4`.

### [CybExplorer](https://github.com/c1cad4/CybExplorer)

Просмотр блоков и транзакций.

Статус: Архитектурный план: только README и LICENSE. Категория: markets.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybGrowth](https://github.com/c1cad4/CybGrowth)

Развитие людей, обучение, персональные наставники и полезные миссии..

Статус: Архитектурный план: только README и LICENSE. Категория: experience.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [cybguard](https://github.com/c1cad4/cybguard)

Проверка ограниченных действий локального агента.

Статус: library. Категория: security.
Проверка: `cargo test --locked -j 4`.

### [CybIdentity](https://github.com/c1cad4/CybIdentity)

Криптографическая идентичность участников.

Статус: Архитектурный план: только README и LICENSE. Категория: security.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybLab](https://github.com/c1cad4/CybLab)

Изолированные эксперименты.

Статус: Архитектурный план: только README и LICENSE. Категория: experience.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [cybLaunch](https://github.com/c1cad4/cybLaunch)

CLI для явного запуска проверок и статических приложений.

Статус: tool. Категория: system.
Проверка: `python3 -m unittest discover -s tests`.

### [CybMemory](https://github.com/c1cad4/CybMemory)

Хранение и поиск знаний.

Статус: library. Категория: intelligence.
Проверка: `python3 -m unittest discover -s tests`.

### [CybMentor](https://github.com/c1cad4/CybMentor)

Персональный ИИ-наставник.

Статус: Архитектурный план: только README и LICENSE. Категория: intelligence.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybMind](https://github.com/c1cad4/CybMind)

Коллективный интеллект, рои ИИ-агентов, исследования и координация задач..

Статус: Архитектурный план: только README и LICENSE. Категория: intelligence.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybMissions](https://github.com/c1cad4/CybMissions)

Полезные задания, обучение и подтверждение достижений.

Статус: Архитектурный план: только README и LICENSE. Категория: experience.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [cybnet](https://github.com/c1cad4/cybnet)

Контракты mesh-пакетов, TTL и конфигурации VPN.

Статус: library. Категория: network.
Проверка: `cargo test --locked -j 4`.

### [cybOS](https://github.com/c1cad4/cybOS)

Сборка всей экосистемы: карта проектов, документация и воспроизводимые рабочие команды.

Статус: orchestrator. Категория: system.
Проверка: `python3 -m unittest discover -s tests`.

### [CybOS-demo](https://github.com/c1cad4/CybOS-demo)

Нативный рабочий стол и тестовый стенд, собирающий самостоятельные компоненты.

Статус: application. Категория: system.
Подключает: robotcyb-core, cybnet, cybguard, cybbee, cybchain, cybweb, CybBrowser, cybdex, soul, immunocybchain.
Проверка: `cargo test --locked -j 4`.

### [CybRegistry](https://github.com/c1cad4/CybRegistry)

Каталог и проверка агентов.

Статус: library. Категория: intelligence.
Проверка: `python3 -m unittest discover -s tests`.

### [CybRobot](https://github.com/c1cad4/CybRobot)

Интеграция роботов, адаптеры оборудования, симуляция и безопасное управление..

Статус: Архитектурный план: только README и LICENSE. Категория: physical.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybShield](https://github.com/c1cad4/CybShield)

Криптография, идентичность, разрешения и безопасность всей экосистемы..

Статус: Архитектурный план: только README и LICENSE. Категория: security.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybSwarm](https://github.com/c1cad4/CybSwarm)

Координация и выполнение задач группами агентов.

Статус: library. Категория: intelligence.
Подключает: CybAgents, CybRegistry, CybMemory.
Проверка: `python3 -m unittest discover -s tests`.

### [CybTrust](https://github.com/c1cad4/CybTrust)

Репутация людей, агентов, роботов и узлов на основе проверенных достижений..

Статус: Архитектурный план: только README и LICENSE. Категория: security.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [CybWallet](https://github.com/c1cad4/CybWallet)

Отдельные требования к безопасности ключей, транзакций и тестового кошелька.

Статус: Архитектурный план: только README и LICENSE. Категория: markets.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [cybweb](https://github.com/c1cad4/cybweb)

Извлечение текста HTML и ранжирование источников для поиска.

Статус: library. Категория: network.
Проверка: `cargo test --locked -j 4`.

### [CybWorld](https://github.com/c1cad4/CybWorld)

Цифровой двойник мира: датчики, экосистемы, фермы и граф физических объектов..

Статус: Архитектурный план: только README и LICENSE. Категория: physical.
Граница: Архитектурный проект; реализации и тестов пока нет.

### [Egolution](https://github.com/c1cad4/Egolution)

Браузерная игра эволюции до RobotCYB с отдельным серверным учётом косметики.

Статус: application. Категория: experience.
Граница: Аккаунты и платная косметика требуют отдельного backend; локальные тесты не подтверждают сетевые платежи.
Проверка: `node --test`.

### [immunocybchain](https://github.com/c1cad4/immunocybchain)

Идентичность сообщений и ограниченное окно защиты от повторов.

Статус: library. Категория: security.
Проверка: `cargo test --locked -j 4`.

### [robotcyb](https://github.com/c1cad4/robotcyb)

Публичная история RobotCYB и входная страница проекта.

Статус: website. Категория: intelligence.
Граница: Проверка интерфейса выполняется браузерным smoke-тестом.

### [robotcyb-core](https://github.com/c1cad4/robotcyb-core)

Контракты рабочих задач, дедлайны, Modbus и энергетическая политика.

Статус: library. Категория: intelligence.
Проверка: `cargo test --locked -j 4`.

### [Robotcyb-game](https://github.com/c1cad4/Robotcyb-game)

Игровая симуляция фермы с ресурсами и проверяемыми правилами.

Статус: demo. Категория: experience.
Проверка: `node --test`.

### [RobotCYB-Price-Tracker](https://github.com/c1cad4/RobotCYB-Price-Tracker)

Браузерное расширение для общей цены токена и виджета.

Статус: extension. Категория: markets.
Граница: Полная загрузка расширения требует браузера, разрешающего unpacked extensions.
Проверка: `node --test`.

### [soul](https://github.com/c1cad4/soul)

Типы локальной памяти, фактов обучения и графа знаний.

Статус: library. Категория: intelligence.
Проверка: `cargo test --locked -j 4`.

### [soul-demo](https://github.com/c1cad4/soul-demo)

Интерактивная WebGL-визуализация: отдельный визуальный эксперимент.

Статус: demo. Категория: experience.
Граница: Нужен WebGL-браузер; микрофон требует разрешения.

## Исторические записи и область проверки

В старом каталоге также есть go-cyber и cyb. Их нет в текущей публичной выдаче и списке доступных репозиториев GitHub-подключения. Это не доказательство удаления: видимость или права могли измениться. Сохранены как исторические записи; их код и сборка в этой проверке не подтверждены.

Полные device/network/платёжные проверки не заменяются unit-тестами. cybchat требует отдельной проверки Xcode/Apple SDK; soul-demo проверен по README/структуре, без запуска WebGL и микрофона.

Список тестов и план разработки рабочего стола находятся в [DESKTOP_DEVELOPMENT.md](DESKTOP_DEVELOPMENT.md).
