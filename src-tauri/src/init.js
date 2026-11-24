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
    // 22-nov-2026: asistente local (Ollama en esta computadora). Lo usa el agente del panel (app/sistema/agente-widget.php).
    if (!android) {
        window.NIDO_APP.ia = 1;
        window.nidoIa = {
            estado: function () { return invocar('ia_estado'); },
            chat: function (cuerpo) { return invocar('ia_chat', { cuerpo: cuerpo }); },
            configurar: function () { return invocar('abrir_asistente'); }
        };
    }
    // 24-nov-2026: si la pantalla anti-robots del hosting («Espere mientras se verifica su solicitud…») no avanza en 25 s,
    // se explica qué pasa y se ofrece reintentar (antes se quedaba cargando sin decir nada)
    (function () {
        function revisar() {
            var t = ((document.title || '') + ' ' + (document.body ? document.body.innerText.slice(0, 400) : '')).toLowerCase();
            if (!/verifica su solicitud|verifying your request|checking your browser|verificando|just a moment/.test(t)) return;
            setTimeout(function () {
                var t2 = ((document.title || '') + ' ' + (document.body ? document.body.innerText.slice(0, 400) : '')).toLowerCase();
                if (!/verifica su solicitud|verifying your request|checking your browser|verificando|just a moment/.test(t2) || document.getElementById('nidoVerifica')) return;
                var d = document.createElement('div');
                d.id = 'nidoVerifica';
                d.setAttribute('style', 'position:fixed;left:50%;bottom:24px;transform:translateX(-50%);z-index:2147483647;max-width:min(520px,calc(100vw - 32px));'
                    + 'padding:16px 18px;border-radius:14px;background:#111;color:#f2f4f7;font:14px/1.5 system-ui,-apple-system,sans-serif;box-shadow:0 18px 40px rgba(0,0,0,.45);text-align:left');
                d.innerHTML = '<strong style="display:block;margin-bottom:4px">La protección del hosting está revisando esta conexión</strong>'
                    + 'Si no avanza, el firewall del hosting puede estar frenando la red de esta computadora (por ejemplo, después de muchas consultas seguidas). '
                    + 'Espera unos minutos y reintenta, prueba con otra red, o pide a tu hosting que permita esta IP.'
                    + '<div style="margin-top:12px;text-align:right"><button type="button" style="font:600 13px system-ui;padding:8px 14px;border-radius:9px;border:0;background:#f2f4f7;color:#111;cursor:pointer">Reintentar</button></div>';
                d.querySelector('button').addEventListener('click', function () { location.reload(); });
                document.body.appendChild(d);
            }, 25000);
        }
        if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', revisar); else revisar();
    })();
    // 22-nov-2026 (pedido de Erick): aviso al terminar cada descarga con la carpeta donde quedó (lo llama lib.rs → on_download)
    window.nidoDescargaLista = function (ruta, ok) {
        var nombre = String(ruta || '').split(/[\\/]/).pop();
        var titulo = ok ? 'Descarga lista' : 'La descarga no se completó';
        var texto = ok ? (nombre ? nombre + ' se guardó en:' : 'El archivo se guardó en:') : 'Intenta otra vez.';
        if (typeof window.showEstiloAlerta === 'function') {
            window.showEstiloAlerta({
                variant: ok ? 'success' : 'error', icon: ok ? 'bi-download' : 'bi-x-circle', title: titulo, message: texto,
                bodyHtml: ok && ruta ? '<p style="margin:8px 0 0;padding:10px 12px;border-radius:12px;background:rgba(127,127,127,.12);font:13px ui-monospace,Menlo,monospace;word-break:break-all;text-align:left">'
                    + String(ruta).replace(/[&<>]/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]; }) + '</p>' : '',
                confirmText: ok && !android ? 'Abrir carpeta' : 'Listo',
                secondaryText: ok && !android ? 'Cerrar' : '',
                onConfirm: function () { if (ok && !android) invocar('abrir_descargas').catch(function () {}); }
            });
        } else {
            window.alert(titulo + '\n' + texto + (ok && ruta ? '\n' + ruta : ''));
        }
    };
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
