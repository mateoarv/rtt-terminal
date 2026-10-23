# Instrucciones y Guía para Agentes de IA (`AGENTS.md`)

Este repositorio (`rtt-terminal`) utiliza este documento para definir las directrices de diseño, reglas de desarrollo y protocolos obligatorios que cualquier agente de IA o colaborador automatizado debe acatar al trabajar en el proyecto.

---

## ⚠️ Regla Obligatoria: Mantenimiento Continuo de `README.md`

> [!IMPORTANT]
> **El archivo [`README.md`](file:///c:/Users/mateo/Repos/Samay/rtt-terminal/README.md) DEBE actualizarse inmediatamente cada vez que se implemente, modifique o elimine cualquier funcionalidad en el proyecto.**

Todo agente que realice cambios de código debe revisar y sincronizar el `README.md` como parte indivisible de su tarea:
1. **Nuevos argumentos o flags de CLI:** Reflejar cualquier cambio en `src/cli.rs` en la tabla de comandos o ejemplos de uso de `README.md`.
2. **Nuevos comandos slash (`/`) o atajos:** Documentar en las secciones correspondientes de la TUI si se agregan comandos como `/raw`, `/hex`, `/eol`, etc.
3. **Nuevos backends, dependencias o modos de ejecución:** Actualizar la tabla tecnológica y las instrucciones de compilación o ejecución.
4. **Avance en el roadmap:** Marcar como completadas o en progreso las etapas en la sección *Estado del Desarrollo y Roadmap* en concordancia con [`STAGES.md`](file:///c:/Users/mateo/Repos/Samay/rtt-terminal/STAGES.md).
5. **No concluir una tarea de implementación** sin haber verificado que la documentación pública (`README.md`) y técnica describe con total fidelidad el estado actual del código.

---

## 🧭 Visión y Arquitectura del Proyecto

`rtt-terminal` es una terminal RTT (Real Time Transfer) interactiva de alto rendimiento en Rust (edición 2024, MSRV 1.85+) para microcontroladores mediante sondas Segger J-Link, CMSIS-DAP y ST-Link.

### Componentes Principales:
- **`src/cli.rs`:** Definición tipada de la CLI con `clap`.
- **`src/probe/`:** Capa de comunicación de bajo nivel.
  - `detector.rs`: Descubrimiento de sondas USB.
  - `targets.rs`: Catálogo y búsqueda difusa de microcontroladores soportados por `probe-rs`.
  - `segger.rs`: Backend nativo con la DLL de Segger J-Link (`libloading`) para coexistencia y multiplexación sin conflictos.
  - `session.rs` & `rtt.rs`: Conexión de depuración y enlace con el bloque `_SEGGER_RTT` en la RAM del microcontrolador.
  - `worker.rs`: Hilo en segundo plano que sondea datos RTT y se comunica con la TUI mediante canales `crossbeam-channel` sin bloquear el hilo principal.
- **`src/tui/`:** Interfaz de usuario basada en `ratatui` y `crossterm`.
  - `app.rs`: Estado reactivo (`AppState`), historial de comandos, buffer circular de logs (10,000 entradas) y cálculo de scroll.
  - `ansi.rs`: Parser de secuencias ANSI (colores 16/256/RGB, borrado y movimiento de cursor).
  - `events.rs`: Procesamiento de eventos de teclado, ratón y despacho de comandos slash.
  - `ui.rs`: Renderizado del layout a 60 FPS.
- **`src/mock.rs`:** Generador de telemetría simulada para pruebas visuales desacopladas del hardware físico (`--mock`).
- **`src/logging.rs`:** Volcado asíncrono y limpio de logs a disco sin retrasar el renderizado.

---

## 📋 Protocolo de Desarrollo para Agentes

Al recibir una tarea o requerimiento:

1. **Consultar Requerimientos y Etapas:**
   - Leer [`REQUIREMENTS.md`](file:///c:/Users/mateo/Repos/Samay/rtt-terminal/REQUIREMENTS.md) para entender el alcance y especificación funcional.
   - Revisar [`STAGES.md`](file:///c:/Users/mateo/Repos/Samay/rtt-terminal/STAGES.md) para ubicar en qué etapa se encuentra el desarrollo y los criterios de validación.

2. **Verificar Estado y Ejecutar Tests Previos:**
   - Ejecutar `cargo test` para asegurar que el workspace esté limpio y sin regresiones antes de tocar código.

3. **Implementación de Código:**
   - Escribir código idiomático en Rust, aprovechando el sistema de tipos y control estricto de concurrencia.
   - Manejar errores con `anyhow::Result` evitando panics innecesarios (`unwrap()` o `expect()` injustificados).
   - Mantener la integridad de los comentarios y docstrings existentes.
   - Garantizar que los hilos no bloqueen el renderizado de la TUI.

4. **Validación:**
   - Agregar tests unitarios para nuevas funciones o parsers.
   - Comprobar que compila sin errores (`cargo check` / `cargo build`).
   - Comprobar que todos los tests pasen (`cargo test`).
   - Si se modificó la interfaz o interacción, probar en modo simulado (`cargo run -- --mock`).

5. **Actualización de Documentación (Obligatorio):**
   - Actualizar [`README.md`](file:///c:/Users/mateo/Repos/Samay/rtt-terminal/README.md) reflejando las nuevas funciones, argumentos, comandos o cambios de arquitectura.
   - Si se completó una etapa de [`STAGES.md`](file:///c:/Users/mateo/Repos/Samay/rtt-terminal/STAGES.md), actualizar también el checklist de seguimiento.
