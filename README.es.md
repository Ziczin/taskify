# Taskify

[English](README.en.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Українська](README.uk.md) · [Polski](README.pl.md) · **Español** · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify es una aplicación local de gamificación de tareas personales para escritorio. Gestiona hábitos, tareas repetitivas y puntuales, y listas de comprobación; gana XP y monedas; desbloquea logros, temas, cosméticos y pegatinas. Todo se guarda en tu equipo en un diario de eventos JSONL: sin cuenta y sin red.

![Pantalla «Hoy»](docs/presentation/today.png)

## Características

- Cuatro tipos de tareas: hábitos, repetitivas, puntuales y listas con subtareas.
- Niveles sin límite, progreso diario y semanal, rachas, combos y logros.
- Monedas, tienda de cosméticos, temas, acentos, fondos, fuentes, sonidos y pegatinas arrastrables.
- Paquetes de contenido en TOML: tareas, categorías, etiquetas, iconos, paletas, combos, logros, cosméticos y conjuntos.
- Varios livespaces: cada uno tiene su propia historia, compras y tablero de pegatinas.
- Ocho idiomas de interfaz: ruso, inglés, japonés, ucraniano, polaco, español, chino y alemán.
- Almacenamiento solo local: diario de eventos, cachés reconstruibles y copias de seguridad automáticas.
- Modo portátil mediante `TASKIFY_HOME`.

## Requisitos

- Windows 10 o posterior, macOS 11 o posterior, o Linux con GTK3, X11/Wayland y OpenGL.
- Rust 1.85 o posterior.
- Herramientas de compilación de C++ si tu instalación de Rust aún no las tiene.

## Compilación y ejecución

```powershell
cargo build --release
.\tools\package.ps1          # Windows
.\target\release\taskify.exe
```

```bash
cargo build --release
./tools/package.sh           # Linux: tar.gz; macOS: Taskify.app en ZIP
./target/release/taskify
```

El mismo ejecutable ofrece comandos de línea de comandos:

```bash
target/release/taskify version
target/release/taskify packs
target/release/taskify stats default
target/release/taskify validate history default
```

La compilación de macOS está firmada ad-hoc, no notarizada: si Gatekeeper la bloquea, usa clic derecho → Abrir la primera vez.

El primer inicio crea el livespace `default` e instala los paquetes integrados.

## Datos

Por defecto, todo está en `%APPDATA%\taskify`:

- `livespaces\<nombre>` — diario `data/log/YYYY-MM.jsonl`, cachés `state.json` y `aggregates.json`, copias de seguridad.
- `packs\<id>` — paquetes de contenido instalados.
- `settings.toml` — ajustes de la ventana y de la aplicación.

La variable `TASKIFY_HOME` traslada todo el directorio de datos a otro lugar (modo portátil). El diario es la fuente de verdad: las cachés se pueden borrar y reconstruir a partir de los eventos.

## Línea de comandos

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <ruta-del-paquete>
taskify validate all [<data-root>]
taskify validate history <livespace>
taskify stats <livespace>
taskify log <livespace> [--limit N]
taskify rebuild <livespace>
taskify backup <livespace>
taskify import <archivo.zip> <livespace> [<data-root>]
taskify export <livespace> <archivo.zip>
```

## Estructura del proyecto

| Crate | Responsabilidad |
| --- | --- |
| `crates/core` | Tareas, recompensas, curva de niveles, métricas, límites del día. |
| `crates/store` | Eventos, diario, cachés, reconstrucción, copias de seguridad, livespaces. |
| `crates/packs` | Carga de paquetes TOML, registros, validador, instalación zip. |
| `crates/ui` | Ventana egui/eframe, pantallas, reglas de juego, localización, CLI. |


## Paquetes

Un paquete es una carpeta con `pack.toml` y subcarpetas para tareas, combos, logros, cosméticos, conjuntos, definiciones y multimedia. Los identificadores llevan el prefijo del paquete (`greeting/hello`). Los paquetes integrados están en `assets/packs` y se instalan en el primer inicio.

## Comprobaciones

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

El conjunto actual tiene **183 pruebas**.

## Contribuir

Las issues y los pull requests son bienvenidos. Antes de abrir un pull request, ejecuta las comprobaciones anteriores y traduce todo el texto visible a cada idioma de `crates/ui/src/i18n.rs`.

## Licencia

Taskify se distribuye bajo la licencia MIT. Consulta [LICENSE](LICENSE). Las fuentes Noto incluidas están bajo la SIL Open Font License; consulta `assets/fonts/LICENSE-OFL.txt`.
