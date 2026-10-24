import json
from pathlib import Path
import sys
import tempfile

import gi

gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import GLib, Gtk, WebKit2


raiz = Path(__file__).resolve().parents[1]
fuentes = raiz / 'src'
codigo = (fuentes / 'main.js').read_text().split('// ───────── BOOT ─────────')[0]
# Se usa el editor real y se sustituye únicamente el IPC para no escribir notas
# del usuario ni arrancar ventanas auxiliares durante las pruebas.
html = (fuentes / 'index.html').read_text().replace(
    '<script src="main.js" type="module"></script>',
    '<script>' + codigo.replace('</script', '<\\/script') + '</script>')
pruebas = (raiz / 'tests' / 'editor.js').read_text()
salida = Path(tempfile.mkdtemp(prefix='notepad-editor-'))
ventana = Gtk.Window()
ventana.set_default_size(1100, 780)
vista = WebKit2.WebView()
ventana.add(vista)
estado = {'codigo': 1}


def captura_fin(vista, resultado, datos):
    try:
        ruta = salida / 'editor.png'
        vista.get_snapshot_finish(resultado).write_to_png(str(ruta))
        print('Captura:', ruta, flush=True)
        estado['codigo'] = 0
    except Exception as error:
        print('Error de captura:', error, flush=True)
    Gtk.main_quit()


def pruebas_fin(vista, resultado, datos):
    try:
        valor = vista.call_async_javascript_function_finish(resultado)
        print(json.dumps(json.loads(valor.to_string()), ensure_ascii=False, indent=2), flush=True)
        vista.get_snapshot(WebKit2.SnapshotRegion.VISIBLE, WebKit2.SnapshotOptions.NONE,
                           None, captura_fin, None)
    except Exception as error:
        print('Error de prueba:', error, flush=True)
        Gtk.main_quit()


def cargado(vista, evento):
    if evento == WebKit2.LoadEvent.FINISHED:
        vista.call_async_javascript_function(pruebas, -1, None, None, None, None,
                                             pruebas_fin, None)


def agotar_tiempo():
    print('Tiempo de prueba agotado', flush=True)
    Gtk.main_quit()
    return False


vista.connect('load-changed', cargado)
ventana.show_all()
vista.load_html(html, fuentes.as_uri() + '/')
GLib.timeout_add_seconds(45, agotar_tiempo)
Gtk.main()
sys.exit(estado['codigo'])
