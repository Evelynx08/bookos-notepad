const resultados = [];
function comprobar(condicion, mensaje) {
  if (!condicion) throw new Error(mensaje);
  resultados.push(mensaje);
}
const cuadro = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
const escrituras = [];
let escritura;
window.__TAURI__ = {
  core: { invoke: async (comando, argumentos) => {
    if (comando === 'write_file') {
      escrituras.push(argumentos);
      if (escritura) return escritura(argumentos);
    }
  } },
  window: { getCurrentWindow: () => ({ setTitle() {} }) }
};
wireKeys();
const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="800"><rect width="1600" height="800" fill="#e5c18c"/><path d="M0 0L1600 800M1600 0L0 800" stroke="#6d5842" stroke-width="25"/></svg>';
const fuente = 'data:image/svg+xml,' + encodeURIComponent(svg);
const nota = crearNota('/tmp/primera.html', '<div>Primera nota</div><img style="width: 600px" src="' + fuente + '"><div>Texto después de la imagen</div>' + '<div>Línea larga de prueba</div>'.repeat(60), Date.now()/1000, true);
const otra = crearNota('/tmp/segunda.html', '<div>Segunda nota</div><div>Texto <b>con formato</b> para recordar</div>' + '<div>Otra línea de prueba</div>'.repeat(60), Date.now()/1000, true);
$('#mc').classList.add('sin-lista');
setActive(nota.id);
const editor = () => $('.editor-area');
const imagen = () => editor().querySelector('img');
await imagen().decode(); await cuadro();
const textoOriginal = htmlToPlain(editor().innerHTML);
comprobar($('.nota-guardado').textContent === 'Guardado', 'Una nota leída del disco aparece guardada');
imagen().click();
comprobar($('.img-acciones').querySelectorAll('button').length === 5, 'La imagen ofrece tamaño original, ajuste y tres alineaciones');
comprobar(document.querySelectorAll('.img-asa').length === 4, 'Se conservan los cuatro tiradores');

// La captura de un puntero físico no se puede iniciar con eventos sintéticos.
const capturar = Element.prototype.setPointerCapture;
Element.prototype.setPointerCapture = function() {};
async function arrastrar(esquina, dx, dy) {
  const asa = $('.img-asa.' + esquina);
  asa.dispatchEvent(new PointerEvent('pointerdown', {pointerId: 1, button: 0, clientX: 700, clientY: 450}));
  asa.dispatchEvent(new PointerEvent('pointermove', {pointerId: 1, clientX: 700 + dx/2, clientY: 450 + dy/2}));
  asa.dispatchEvent(new PointerEvent('pointermove', {pointerId: 1, clientX: 700 + dx, clientY: 450 + dy}));
  asa.dispatchEvent(new PointerEvent('pointerup', {pointerId: 1}));
  clearTimeout(nota.temporizador);
  await imagen().decode(); await cuadro();
}
function atajo(tecla, mayusculas = false) {
  editor().dispatchEvent(new KeyboardEvent('keydown', {key: tecla, ctrlKey: true, shiftKey: mayusculas, bubbles: true, cancelable: true}));
  clearTimeout(nota.temporizador);
}
await arrastrar('inf-der', -180, -90);
comprobar(imagen().style.width === '420px', 'El arrastre cambia el tamaño');
comprobar($('.nota-guardado').textContent === 'Cambios pendientes', 'El cambio queda pendiente de guardar');
atajo('z');
comprobar(imagen().style.width === '600px', 'Ctrl+Z deshace todo el arrastre de una vez');
atajo('z', true);
comprobar(imagen().style.width === '420px', 'Ctrl+Shift+Z rehace el tamaño');

const rango = document.createRange();
rango.selectNodeContents(editor().querySelector('div:last-child')); rango.collapse(false);
const seleccion = window.getSelection(); seleccion.removeAllRanges(); seleccion.addRange(rango);
document.execCommand('insertText', false, ' añadido'); clearTimeout(nota.temporizador);
atajo('z');
comprobar(!editor().textContent.endsWith(' añadido') && imagen().style.width === '420px', 'Deshacer el texto posterior no revierte antes la imagen');
atajo('z');
comprobar(imagen().style.width === '600px', 'El siguiente deshacer recupera el tamaño anterior');
atajo('y');
comprobar(imagen().style.width === '420px', 'Ctrl+Y también rehace el redimensionado');

async function accion(nombre) {
  imagen().click();
  $('.img-acciones [data-accion="' + nombre + '"]').click();
  clearTimeout(nota.temporizador);
  await imagen().decode(); await cuadro();
}
await accion('original');
comprobar(imagen().style.width === '1600px' && imagen().width <= editor().clientWidth, 'Tamaño original conserva la resolución sin desbordar el editor');
await accion('ajustar');
comprobar(imagen().width === Math.round(anchoDisponibleImagen(imagen())), 'Ajustar al ancho utiliza el espacio de la nota');
await accion('centro');
comprobar(imagen().style.marginLeft === 'auto' && imagen().style.marginRight === 'auto', 'Centrar cambia los márgenes de la imagen');
atajo('z');
comprobar(imagen().style.marginLeft === '', 'La alineación también se puede deshacer');
atajo('y');
comprobar(imagen().style.marginLeft === 'auto' && imagen().style.marginRight === 'auto', 'La alineación también se puede rehacer');
await accion('derecha');
comprobar(Math.abs(imagen().getBoundingClientRect().right - (editor().getBoundingClientRect().left + editor().clientWidth - parseFloat(getComputedStyle(editor()).paddingRight))) < 1, 'La imagen se alinea a la derecha');
await accion('izquierda');
comprobar(Math.abs(imagen().getBoundingClientRect().left - editor().getBoundingClientRect().left - parseFloat(getComputedStyle(editor()).paddingLeft)) < 1, 'La imagen se alinea a la izquierda');
editor().dispatchEvent(new KeyboardEvent('keydown', {key: 'Tab', shiftKey: true, bubbles: true}));
comprobar(!!$('.img-acciones'), 'Tab permite acceder a las acciones sin cerrar la selección');
const centrar = $('.img-acciones [data-accion="centro"]');
centrar.focus(); centrar.click(); clearTimeout(nota.temporizador);
await imagen().decode(); await cuadro();
comprobar(document.activeElement.dataset.accion === 'centro', 'Una acción conserva el foco para seguir usando el teclado');
await arrastrar('sup-izq', 120, 60);
const ancho = imagen().width;
await arrastrar('sup-der', -60, 30);
comprobar(imagen().width < ancho, 'Se puede seguir arrastrando desde otras esquinas después de las acciones');
editor().scrollTop = 200; await cuadro();
comprobar(marcoImagen && Math.abs($('.img-marco').getBoundingClientRect().top - imagen().getBoundingClientRect().top) < 1, 'El scroll conserva y recoloca el marco');
comprobar(htmlToPlain(editor().innerHTML) === textoOriginal, 'Cambiar la imagen no modifica el texto ni añade líneas a la nota');
saveActiveContent();
comprobar(!nota.content.includes('img-acciones') && !nota.content.includes('img-marco'), 'Las acciones y el marco no se escriben dentro de la nota');
const texto = editor().querySelector('div:nth-of-type(15)').firstChild;
seleccion.setBaseAndExtent(texto, 8, texto, 3);
editor().scrollTop = 460;
const desplazamiento = editor().scrollTop;
setActive(otra.id); await cuadro();
const negrita = editor().querySelector('b').firstChild;
seleccion.setBaseAndExtent(negrita, 4, negrita, 4); editor().scrollTop = 280;
const desplazamientoOtra = editor().scrollTop;
setActive(nota.id); await imagen().decode(); await cuadro();
comprobar(editor().scrollTop === desplazamiento, 'Al volver a la primera nota se recupera el scroll');
comprobar(seleccion.anchorOffset === 8 && seleccion.focusOffset === 3 && seleccion.toString() === 'ea la', 'Se recupera incluso una selección de texto hacia atrás');
setActive(otra.id); await cuadro();
comprobar(editor().scrollTop === desplazamientoOtra && seleccion.anchorNode.parentElement.tagName === 'B' && seleccion.anchorOffset === 4, 'La segunda nota conserva su cursor dentro del formato y su propio scroll');
showPage('settings'); showPage('editor'); await cuadro();
comprobar(editor().scrollTop === desplazamientoOtra && seleccion.anchorOffset === 4, 'Abrir y cerrar ajustes también conserva la posición');
setActive(nota.id); await imagen().decode(); await cuadro();

let completar;
escritura = () => new Promise(resolve => { completar = resolve; });
editor().querySelector('div:last-child').textContent = 'Versión primera';
onEditorInput(); clearTimeout(nota.temporizador);
const guardado = guardarNota(nota);
comprobar($('.nota-guardado').textContent === 'Guardando…', 'Durante la escritura se muestra Guardando…');
const cantidad = escrituras.length;
editor().querySelector('div:last-child').textContent = 'Versión segunda';
onEditorInput(); clearTimeout(nota.temporizador);
const simultaneo = guardarNota(nota);
comprobar(escrituras.length === cantidad, 'Guardar otra vez no inicia una escritura simultánea de la misma nota');
completar(); await cuadro();
comprobar(escrituras.length === cantidad + 1 && escrituras.at(-1).contents.includes('Versión segunda'), 'La escritura pendiente incluye la versión más reciente');
comprobar($('.nota-guardado').textContent === 'Guardando…', 'La primera respuesta no anuncia Guardado mientras queda contenido pendiente');
completar(); await Promise.all([guardado, simultaneo]);
comprobar($('.nota-guardado').textContent === 'Guardado' && !nota.dirty, 'Solo la última escritura confirmada muestra Guardado');

escritura = async () => { throw new Error('Disco de prueba lleno'); };
onEditorInput(); clearTimeout(nota.temporizador); await guardarNota(nota);
comprobar($('.nota-guardado').textContent === 'Error al guardar' && !$('.nota-guardado').disabled && nota.dirty, 'Un fallo mantiene los cambios pendientes y ofrece reintento');
comprobar($('.nota-guardado').title.includes('Disco de prueba lleno'), 'El reintento explica el motivo del fallo');
setActive(otra.id); setActive(nota.id); await imagen().decode(); await cuadro();
comprobar($('.nota-guardado').textContent === 'Error al guardar', 'El error de una nota se conserva al regresar a ella');
escritura = undefined;
$('.nota-guardado').click(); await cuadro();
comprobar($('.nota-guardado').textContent === 'Guardado' && !nota.dirty, 'Pulsar el estado de error reintenta y confirma el guardado');
const nueva = crearNota('/tmp/nueva.html', '', Date.now()/1000, false);
setActive(nueva.id);
comprobar($('.nota-guardado').textContent === 'Sin guardar', 'Una nota nueva no anuncia un guardado inexistente');
const antes = escrituras.length;
onEditorInput(); clearTimeout(nueva.temporizador); await guardarNota(nueva);
comprobar(escrituras.length === antes && $('.nota-guardado').textContent === 'Sin guardar', 'Una nota vacía no crea un archivo ni se queda guardando');
setActive(otra.id); await cuadro();
const bloque = editor().querySelector('div:nth-of-type(10)');
bloque.textContent = 'Texto unido';
bloque.firstChild.splitText(5);
seleccion.setBaseAndExtent(bloque.lastChild, 3, bloque.lastChild, 3);
saveActiveContent();
setActive(nota.id); setActive(otra.id); await cuadro();
comprobar(seleccion.anchorNode.textContent === 'Texto unido' && seleccion.anchorOffset === 8, 'El cursor se restaura aunque guardar una los nodos de texto contiguos');
const vacio = document.createElement('div'); vacio.appendChild(document.createElement('br'));
editor().appendChild(vacio); seleccion.setBaseAndExtent(vacio, 0, vacio, 0);
saveActiveContent();
setActive(nota.id); setActive(otra.id); await cuadro();
comprobar(seleccion.anchorNode === editor().lastChild && seleccion.anchorOffset === 0, 'El cursor también vuelve a una línea vacía');
setActive(nota.id); await imagen().decode(); await cuadro();
imagen().click();
const anchoPrevio = imagen().style.width;
const asa = $('.img-asa.inf-der');
asa.dispatchEvent(new PointerEvent('pointerdown', {pointerId: 1, button: 0, clientX: 500, clientY: 400}));
asa.dispatchEvent(new PointerEvent('pointermove', {pointerId: 1, clientX: 400, clientY: 350}));
setActive(otra.id); setActive(nota.id); await imagen().decode(); await cuadro();
comprobar(imagen().style.width === anchoPrevio, 'Cambiar de nota cancela un arrastre sin terminar en lugar de guardar una previsualización');
imagen().click(); await arrastrar('inf-der', 420 - imagen().width, 0);
await accion('centro'); await guardarNota(nota);
editor().scrollTop = 0; await cuadro();
await new Promise(resolve => setTimeout(resolve, 1700));
Element.prototype.setPointerCapture = capturar;
return JSON.stringify({resultados, comprobaciones: resultados.length});
