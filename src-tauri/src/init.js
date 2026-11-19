// Corre al inicio de cada página dentro de la app (también las del servidor)
(function () {
    if (window.__nidoAppIniciada) return;
    window.__nidoAppIniciada = true;
    // El panel lo usa para mostrar el engranaje (app/escritorio/engranaje.php). config: 2 = ventana de configuración
    // pública (sin PIN, comando abrir_config); las apps viejas no lo traen y ahí el engranaje sigue solo para el SuperUser.
    // android: true en la app de teléfono (sin engranaje: allí no hay ventanas que ajustar).
    var android = /Android/i.test(navigator.userAgent);
    window.NIDO_APP = { version: '__VERSION__', mac: !android && navigator.userAgent.indexOf('Mac') !== -1, android: android, config: android ? 0 : 2 };
    function invocar(cmd, args) {
        try { return window.__TAURI_INTERNALS__.invoke(cmd, args || {}); } catch (e) { return Promise.reject(e); }
    }
    // Mac: WKWebView no imprime con window.print(); se abre el diálogo nativo
    if (window.NIDO_APP.mac) {
        window.print = function () { invocar('imprimir').catch(function () {}); };
    }
    // Pestañas nuevas: del mismo servidor → otra ventana de la app; de otros sitios → navegador normal
    var abrirOriginal = window.open;
    window.open = function (url) {
        if (!url || url === 'about:blank') return abrirOriginal ? abrirOriginal.apply(window, arguments) : null;
        var destino;
        try { destino = new URL(url, location.href).href; } catch (e) { return null; }
        invocar('abrir_enlace', { url: destino }).catch(function () { location.href = destino; });
        return null;
    };
    document.addEventListener('click', function (e) {
        var a = e.target && e.target.closest ? e.target.closest('a[target="_blank"]') : null;
        if (!a || a.hasAttribute('download') || !a.href || a.href.indexOf('javascript:') === 0) return;
        e.preventDefault();
        window.open(a.href);
    }, true);
})();