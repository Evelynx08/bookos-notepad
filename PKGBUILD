pkgname=bookos-notepad
pkgver=0.1.0
pkgrel=1
pkgdesc="Bloc de notas con pestañas, preview HTML/Markdown/URL y adblock para BookOS"
arch=('x86_64')
url="https://github.com/Evelynx08/bookos-notepad"
license=('MIT')
depends=('webkit2gtk-4.1' 'gtk3' 'libsoup3' 'librsvg' 'openssl' 'ca-certificates' 'xdg-utils')
optdepends=('mpv: reproducción flotante de vídeos YouTube'
            'yt-dlp: requerido por mpv para reproducir YouTube')
makedepends=('rust' 'cargo' 'pkgconf' 'base-devel' 'imagemagick')
source=()
options=('!strip' '!debug')

build() {
  cd "$startdir/src-tauri"
  cargo build --release --locked
}

package() {
  install -Dm755 "$startdir/src-tauri/target/release/bookos-notepad" \
    "$pkgdir/usr/bin/bookos-notepad"

  install -Dm644 /dev/stdin "$pkgdir/usr/share/applications/bookos-notepad.desktop" <<EOF
[Desktop Entry]
Name=Bookos Notepad
GenericName=Editor de texto
Comment=Bloc de notas con pestañas, previsualización y adblock
Exec=bookos-notepad %F
Icon=bookos-notepad
Type=Application
Categories=Utility;TextEditor;Office;
MimeType=text/plain;text/markdown;text/html;text/css;text/x-shellscript;text/x-python;application/json;application/xml;
StartupNotify=true
StartupWMClass=Bookos Notepad
Keywords=editor;notas;notepad;texto;markdown;html;
EOF

  if [ -f "$startdir/src-tauri/icons/icon.png" ]; then
    for sz in 16 22 24 32 48 64 96 128 256 512; do
      install -d "$pkgdir/usr/share/icons/hicolor/${sz}x${sz}/apps"
      magick "$startdir/src-tauri/icons/icon.png" \
        -resize ${sz}x${sz} \
        "$pkgdir/usr/share/icons/hicolor/${sz}x${sz}/apps/bookos-notepad.png" \
        2>/dev/null || \
      convert "$startdir/src-tauri/icons/icon.png" \
        -resize ${sz}x${sz} \
        "$pkgdir/usr/share/icons/hicolor/${sz}x${sz}/apps/bookos-notepad.png"
    done
  fi

  if [ -f "$startdir/LICENSE" ]; then
    install -Dm644 "$startdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
  fi
}
