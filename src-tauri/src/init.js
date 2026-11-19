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
    // «← Volver» (19-nov-2026, pedido de Erick): en pantallas sueltas del servidor (imprimir factura, ticket…) no había cómo
    // volver al panel. Sale abajo a la izquierda si la pantalla no es el panel y no tiene ya su propio «Volver»; no se imprime.
    // (El servidor pone el mismo botón en las pantallas de impresión para las apps viejas: app/sistema/app-volver.php, id #nidoVolver.)
    function ponerVolver() {
        if (location.protocol === 'tauri:' || location.hostname === 'tauri.localhost' || !document.body || document.getElementById('nidoVolver')) return;
        if (/\/(dashboard|cpanel|login|index)\.php$|\/app\/?$/i.test(location.pathname)) return;
        var ya = document.querySelector('.e1-nav-back, [data-volver], [onclick*="history.back"]');
        if (!ya) {
            ya = Array.prototype.some.call(document.querySelectorAll('a, button'), function (el) {
                return /^\s*(←\s*)?volver\b/i.test(el.textContent || '') && el.offsetParent !== null;
            });
        }
        if (ya) return;
        var st = document.createElement('style');
        st.textContent = '#nidoVolver{position:fixed;left:16px;bottom:16px;z-index:99985;display:inline-flex;align-items:center;gap:8px;padding:10px 16px;border:0;'
            + 'border-radius:999px;background:#101828;color:#fff;font:600 14px system-ui,-apple-system,"Segoe UI",sans-serif;cursor:pointer;'
            + 'box-shadow:0 8px 20px rgba(16,24,40,.3)} print{#nidoVolver{display:none!important}}';
        document.head.appendChild(st);
        var b = document.createElement('button');
        b.type = 'button';
        b.id = 'nidoVolver';
        b.innerHTML = '&#8592; Volver';
        b.onclick = function () {
            if (history.length > 1) { history.back(); return; }
            var m = location.pathname.match(/^(.*\/app)(?:\/|$)/i);
            location.href = (m ? m[1] : '') + '/dashboard.php';
        };
        document.body.appendChild(b);
    }
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', ponerVolver); else ponerVolver();
})();
