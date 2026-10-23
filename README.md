# rtt-terminal

> Terminal RTT interactiva de alto rendimiento para microcontroladores mediante sondas de depuración (Segger J-Link, CMSIS-DAP, ST-Link).

---

## 📖 Descripción General

`rtt-terminal` es una herramienta de línea de comandos e interfaz de texto interactiva (TUI) desarrollada en **Rust**. Proporciona comunicación bidireccional en tiempo real con microcontroladores a través de la tecnología **SEGGER RTT (Real Time Transfer)** mediante sondas de depuración.

El proyecto está diseñado para eliminar las fricciones de clientes RTT tradicionales, ofreciendo una experiencia ergonómica, resiliente, con soporte multicanal, decodificación ANSI completa, comandos tipo slash (`/`), modo simulado sin hardware e integración tanto mediante `probe-rs` como mediante la librería oficial de Segger J-Link.

---

## ✨ Características Principales

- **Interfaz TUI Moderna (Ratatui):** Viewport con scroll fluido (teclado y rueda de ratón), barra de estado en vivo (probe, target, canal, estado del core) y línea de entrada interactiva.
- **Soporte de Sondas:** Detección automática y compatibilidad con **Segger J-Link** (enlace nativo DLL y `probe-rs`), **CMSIS-DAP**, y **ST-Link**.
- **Catálogo de Microcontroladores con Búsqueda Difusa:** Búsqueda y validación instantánea de chips compatibles (`nRF52840_xxAA`, `STM32F401RE`, `RP2040`, `ESP32-S3`, etc.).
- **Decodificación ANSI en Tiempo Real:** Renderizado fiel de secuencias de color (16 colores, 256 colores, TrueColor/RGB), estilos (negrita, atenuado) y secuencias de control de cursor (compatibilidad con Zephyr Shell, FreeRTOS CLI).
- **Entrada con Historial y Comandos Slash (`/`):**
  - Envío transparente de líneas de texto al microcontrolador.
  - Comandos internos del sistema (`/help`, `/channel`, `/clear`, `/timestamps`, `/reset`, `/quit`).
  - Navegación de historial con flechas `Arriba` / `Abajo`.
- **Timestamps Configurables:** Marcas de tiempo de alta precisión `[HH:MM:SS.mmm]` conmutables en caliente o vía CLI.
- **Grabación de Sesión (File Logging):** Volcado en vivo y limpio de la sesión a disco en segundo plano sin ralentizar la TUI.
- **Modo Simulado (`--mock`):** Emulador completo con generación continua de logs de prueba para desarrollo y verificación visual sin hardware físico.
- **Modo Headless (`--headless`):** Modo consola ligero directo para automatizaciones y pipelines de CI/CD.

---

## 🛠️ Tecnologías y Dependencias

| Componente | Tecnología | Propósito |
| :--- | :--- | :--- |
| **Lenguaje** | Rust (Edición 2024, MSRV 1.85+) | Concurrencia segura, rendimiento y binario autocontenido |
| **Backend de Hardware** | `probe-rs` (0.32) + Segger DLL (`libloading`) | Enlace y control RTT sobre sondas de depuración |
| **Interfaz de Usuario** | `ratatui` (0.30) + `crossterm` (0.29) | Renderizado TUI reactivo y gestión de eventos |
| **Argumentos CLI** | `clap` (4.6) | Parseo robusto y validación de flags de comando |
| **Búsqueda Difusa** | `fuzzy-matcher` (0.3.7) | Búsqueda rápida en el catálogo de targets |
| **Concurrencia** | `crossbeam-channel` (0.5) | Comunicación lock-free entre el worker RTT y la TUI |
| **Tiempo y Fechas** | `chrono` (0.4) | Timestamps milimétricos locales |

---

## 🚀 Requisitos y Compilación

### Requisitos Previos

- **Rust Toolchain:** Versión 1.85 o superior (compatible con edición Rust 2024).
- Para depuración con hardware físico: Drivers USB correspondientes a la sonda (ej. drivers oficiales de Segger J-Link o drivers WinUSB/CMSIS-DAP).

### Compilación

```bash
# Clonar el repositorio
git clone <url-del-repositorio>
cd rtt-terminal

# Compilación en modo debug
cargo build

# Compilación optimizada para producción
cargo build --release
```

El ejecutable resultante se ubicará en `target/release/rtt-terminal.exe` (Windows) o `target/release/rtt-terminal` (Linux/macOS).

### Ejecutar Pruebas Automatizadas

```bash
cargo test
```

---

## 💻 Guía de Uso

### 1. Comandos de Inspección y Búsqueda

```bash
# Listar sondas de depuración conectadas (J-Link, CMSIS-DAP, etc.)
cargo run -- --list-probes

# Buscar chips compatibles por coincidencia difusa
cargo run -- --search-chip nrf52840
cargo run -- --search-chip stm32f4
```

### 2. Modo Simulado (Sin Hardware)

Ideal para probar la interfaz, scroll, timestamps y atajos:

```bash
# Iniciar TUI con generador de logs simulados
cargo run -- --mock

# Iniciar simulación con timestamps activados y registro a archivo
cargo run -- --mock --timestamps --log sesion_mock.log
```

### 3. Conexión a Hardware Físico

```bash
# Conectar a un chip específico (usa la primera sonda detectada por defecto)
cargo run -- --chip nRF52840_xxAA

# Especificar velocidad de reloj SWD (en kHz) y canal inicial
cargo run -- --chip STM32F401RE --speed 4000 --channel 0

# Conectar a una sonda específica por índice o número de serie
cargo run -- --chip nRF52840_xxAA --probe 0

# Modo Headless (consola sin interfaz gráfica de terminal)
cargo run -- --chip nRF52840_xxAA --headless
```

---

## ⌨️ Atajos y Comandos en la TUI

### Comandos Slash (`/`)

Escribe el comando en la barra inferior y pulsa `Enter`:

| Comando | Parámetros | Descripción |
| :--- | :--- | :--- |
| `/help` | Ninguno | Despliega el resumen de comandos disponibles |
| `/clear` | Ninguno | Limpia el buffer visual de logs en pantalla |
| `/channel`, `/c` | `<0-15>` | Conmuta el canal activo RTT de lectura/escritura |
| `/timestamps` | `[on\|off]` | Activa, desactiva o alterna las marcas de tiempo |
| `/reset` | Ninguno | Ejecuta un reinicio por hardware del microcontrolador |
| `/quit`, `/exit` | Ninguno | Cierra la sesión y restaura la terminal |

### Atajos de Teclado y Navegación

| Tecla / Acción | Función |
| :--- | :--- |
| `Ctrl + C` | Salir de la aplicación inmediatamente |
| `PageUp` / `PageDown` | Desplazar el historial de logs arriba o abajo |
| `Rueda del Ratón` | Scroll vertical en el viewport de logs |
| `Flecha Arriba` / `Flecha Abajo` | Navegar por el historial de comandos enviados |
| `Flecha Izq` / `Der`, `Home`, `End` | Desplazar el cursor en la línea de edición |
| `Esc` | Limpiar la barra de entrada de texto |

---

## 📁 Estructura del Proyecto

```text
rtt-terminal/
├── Cargo.toml            # Dependencias y configuración de Rust 2024
├── REQUIREMENTS.md       # Documento de requerimientos (PRD)
├── STAGES.md             # Plan de implementación incremental por etapas
├── AGENTS.md             # Guía e instrucciones para agentes de IA
├── README.md             # Documentación general del proyecto (este archivo)
└── src/
    ├── main.rs           # Punto de entrada y orquestación
    ├── cli.rs            # Definición de argumentos CLI con Clap
    ├── logging.rs        # Módulo de volcado de logs a disco
    ├── mock.rs           # Generador de telemetría simulada
    ├── probe/            # Capa de hardware y RTT
    │   ├── detector.rs   # Listado y detección de sondas USB
    │   ├── targets.rs    # Base de datos y búsqueda difusa de chips
    │   ├── segger.rs     # Enlace dinámico con DLL Segger J-Link
    │   ├── session.rs    # Gestión de sesión probe-rs
    │   ├── rtt.rs        # Enlace con bloque _SEGGER_RTT
    │   └── worker.rs     # Hilo en segundo plano para E/S RTT
    └── tui/              # Interfaz interactiva de texto (Ratatui)
        ├── ansi.rs       # Parser de secuencias de escape ANSI a estilos
        ├── app.rs        # Estado global de la aplicación (AppState)
        ├── events.rs     # Manejador de eventos de teclado/ratón y slash commands
        ├── ui.rs         # Renderizado de componentes visuales
        └── mod.rs        # Inicialización y ciclo de vida de la TUI
```

---

## 🗺️ Estado del Desarrollo y Roadmap

- [x] **Etapa 1:** CLI, detección de sondas y búsqueda difusa de chips.
- [x] **Etapa 2:** Motor RTT headless básico y canal worker desacoplado.
- [x] **Etapa 3:** Esqueleto de TUI con Ratatui y modo simulado (`--mock`).
- [x] **Etapa 4:** Parser ANSI, timestamps milimétricos y file logging continuo.
- [x] **Etapa 5 (En progreso):** Comandos slash, menú reactivo de autocompletado con `Tab`.
- [ ] **Etapa 6:** Modo Raw interactivo (`/raw`) carácter por carácter para shells embedded y auto-reconexión robusta ante reinicios.
- [ ] **Etapa 7:** Asistente interactivo inicial de selección de chip y empaquetado final de release.