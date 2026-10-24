#!/usr/bin/env bash
set -euo pipefail
RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SALIDA="${BOOKOS_SALIDA_RPM:-$RAIZ/../../Build/bookos-notepad}"
TEMPORAL="$(mktemp -d)"
trap 'rm -rf "$TEMPORAL"' EXIT
cargo build --release --locked --manifest-path "$RAIZ/Cargo.toml"
rpmbuild -bb "$RAIZ/bookos-notepad.spec" \
  --define "_topdir $TEMPORAL" --define "_tmppath $TEMPORAL" --define "raiz $RAIZ" --quiet
mkdir -p "$SALIDA"
# El instalador necesita una única versión del paquete por transacción.
rm -f "$SALIDA"/bookos-notepad-*.rpm
cp "$TEMPORAL"/RPMS/*/bookos-notepad-*.rpm "$SALIDA/"
ls "$SALIDA"/bookos-notepad-*.rpm
