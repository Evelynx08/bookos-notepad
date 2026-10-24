# BookOS Notepad · GPUI
Ejecuta `cargo run` desde la raíz del repositorio o desde `GPUI`, en una terminal del escritorio BookOS (Wayland y Vulkan).
Para abrir archivos: `cargo run -- /ruta/archivo.txt`; si está abierta, los recibe la misma ventana.
`cargo run -- --diagnostico` comprueba Wayland y los programas de integración sin abrir ventanas ni escribir datos.
Si falla, termina con código 1; la disponibilidad de Vulkan se comprueba al abrir la ventana.
Los errores aparecen en la terminal (stderr) y en avisos dentro de la app.
Para una traza de un fallo: `RUST_BACKTRACE=1 cargo run`.
El estado se comparte con Tauri en `~/.config/bookos-notepad/state.json`.
Las notas se guardan en `~/.local/share/bookos-notepad/notas/*.html` (se respetan las variables XDG).
Un estado JSON ilegible se protege contra sobrescrituras; corrígelo o apártalo con la app cerrada.
Ctrl+N: nota; Ctrl+T: archivo; Ctrl+O/S: abrir/guardar; Ctrl+F/H: buscar/reemplazar.
Ctrl+P: vista previa; Ctrl+,: ajustes; Ctrl+/: personalizar atajos; F11: pantalla completa.
El selector necesita `bookos-explorer`; HTML y URL se abren con `xdg-open` en el navegador.
El bloqueo de anuncios se ha retirado de GPUI y Tauri; los ajustes antiguos no activan ningún filtro.
GPUI abre HTML y URL en el navegador; los modos de YouTube y mpv siguen en la versión Tauri.
Las actualizaciones se gestionan con BookOS Settings, sin consultas HTTP propias a GitHub.
Desde `GPUI`, `bash empaquetar.sh` genera el RPM en `../../Build/bookos-notepad/`.
Para otra salida: `BOOKOS_SALIDA_RPM=/tmp/notepad-rpm bash empaquetar.sh`.

El workspace usa el `Cargo.lock` de la raíz y mantiene los binarios en `GPUI/target/`.
Desde la raíz, `bash GPUI/comprobar.sh` comprueba el formato del repositorio, compila, ejecuta Clippy y los tests. Usa las dependencias ya descargadas y el lockfile, sin acceder a la red.
Se usa `cargo fmt -- --check`: `cargo fmt --all` también recorre dependencias locales de otros repositorios, cuyo formato se mantiene fuera de este proyecto.
Las dependencias locales `../plantilla-gpui/bookos-ui` y `../fonts/vendor/gpui` deben estar disponibles junto al repositorio.
La versión anterior se compila por separado con `cargo build --manifest-path src-tauri/Cargo.toml`.

La suite habitual prueba la recepción de rutas y sus errores sin sockets ni Wayland. La prueba de integración real de instancia única se ejecuta por separado con `cargo test segunda_instancia_entrega_rutas -- --ignored`; necesita permiso para crear sockets UNIX.
