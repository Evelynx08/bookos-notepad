# BookOS Notepad · GPUI
Ejecuta `cd GPUI && cargo run` desde una terminal del escritorio BookOS (Wayland y Vulkan).
Para abrir archivos: `cargo run -- /ruta/archivo.txt`; si está abierta, los recibe la misma ventana.
Los errores aparecen en la terminal (stderr) y en avisos dentro de la app.
Para una traza de un fallo: `RUST_BACKTRACE=1 cargo run`.
El estado se comparte con Tauri en `~/.config/bookos-notepad/state.json`.
Las notas se guardan en `~/.local/share/bookos-notepad/notas/*.html` (se respetan las variables XDG).
Un estado JSON ilegible se protege contra sobrescrituras; corrígelo o apártalo con la app cerrada.
Ctrl+N: nota; Ctrl+T: archivo; Ctrl+O/S: abrir/guardar; Ctrl+F/H: buscar/reemplazar.
Ctrl+P: vista previa; Ctrl+,: ajustes; Ctrl+/: personalizar atajos; F11: pantalla completa.
El selector necesita `bookos-explorer`; HTML y URL se abren con `xdg-open` en el navegador.
Se retiraron adblock, modos de YouTube y mpv de GPUI; las claves antiguas se conservan para Tauri.
Las actualizaciones se gestionan con BookOS Settings, sin consultas HTTP propias a GitHub.
`bash empaquetar.sh` genera el RPM en `../../Build/bookos-notepad/`.
Para otra salida: `BOOKOS_SALIDA_RPM=/tmp/notepad-rpm bash empaquetar.sh`.
