# El binario lo compila empaquetar.sh; rpmbuild solo coloca los archivos.
%global debug_package %{nil}
Name:           bookos-notepad
Version:        0.7.0
Release:        1%{?dist}
Summary:        Bloc de notas nativo de BookOS
License:        GPL-3.0-or-later
Requires:       vulkan-loader
Requires:       xdg-utils
Requires:       bookos-explorer
Recommends:     bookos-settings

%description
Editor de texto y notas de BookOS con pestañas, formato, imágenes y vista previa.

%install
install -Dm755 %{raiz}/target/release/bookos-notepad %{buildroot}%{_bindir}/bookos-notepad
install -Dm644 %{raiz}/data/bookos-notepad.desktop %{buildroot}%{_datadir}/applications/bookos-notepad.desktop
install -Dm644 %{raiz}/data/bookos-notepad.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/bookos-notepad.svg
install -Dm644 %{raiz}/data/bookos-notepad.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/bookos-notepad.png

%files
%{_bindir}/bookos-notepad
%{_datadir}/applications/bookos-notepad.desktop
%{_datadir}/icons/hicolor/scalable/apps/bookos-notepad.svg
%{_datadir}/icons/hicolor/512x512/apps/bookos-notepad.png
