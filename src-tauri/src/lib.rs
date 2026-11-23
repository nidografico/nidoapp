// App de escritorio de Nido Gráfico (Tauri 2). Generada por el módulo app/escritorio del panel.
// La ventana carga ui/index.html (pantalla de carga) y de ahí el servidor de app.json (fijo: no se puede cambiar en la app).
// - Enlaces a otros sitios (PayPal, WhatsApp, DGII…) se abren en el navegador normal.
// - Enlaces del mismo servidor con target=_blank / window.open abren otra ventana de la app.
// - Descargas van a la carpeta Descargas. En Mac, window.print() se pasa al diálogo nativo (WKWebView no imprime solo).
// - Engranaje (19-nov-2026, pedido de Erick): lo ve cualquier usuario, sin PIN. Abre la ventana «config» (ui/config.html) con
//   tamaños de ventana ya preparados, tamaño del contenido (90/100/110 %) y herramientas (recargar, ir al inicio, Descargas,
//   valores de fábrica). Ya no hay servidor, PIN ni medidas libres: lo guardado en esta computadora solo puede ser
//   «tamano» y «zoom», y se revisa contra las listas de abajo para que el panel nunca quede en un tamaño que se vea mal.
// Android (19-nov-2026): el mismo proyecto se construye para Android. Lo de ventanas (tamaños, zoom, varias ventanas,
// imprimir, Descargas) es solo de computadora: va marcado con #[cfg(desktop)]. En Android hay una sola pantalla que abre el
// panel; los enlaces de otros sitios van al navegador y los del panel se abren en la misma pantalla.
#![allow(unused_imports, dead_code)]
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
#[cfg(desktop)]
use tauri::{webview::DownloadEvent, LogicalSize};
use tauri_plugin_opener::OpenerExt;

mod ia; // 22-nov-2026: asistente local (Ollama) — ver src/ia.rs

const BASE: &str = include_str!("../../app.json");
const INIT: &str = include_str!("init.js");

/// Tamaños de ventana permitidos: (código, nombre, ancho, alto). Además existe «maximizada».
// 19-nov-2026: las resoluciones más comunes de computadora en las que el panel se ve en su diseño de PC (≥ 1120 × 660).
const TAMANOS: &[(&str, &str, f64, f64)] = &[
    ("hd", "HD · 1280 × 720", 1280.0, 720.0),
    ("compacta", "Compacta · 1280 × 800 (recomendada)", 1280.0, 800.0),
    ("portatil", "Portátil · 1366 × 768", 1366.0, 768.0),
    ("mediana", "Mediana · 1440 × 900", 1440.0, 900.0),
    ("portatil_grande", "Portátil grande · 1536 × 864", 1536.0, 864.0),
    ("hd_plus", "HD+ · 1600 × 900", 1600.0, 900.0),
    ("grande", "Grande · 1600 × 1000", 1600.0, 1000.0),
    ("full_hd", "Full HD · 1920 × 1080", 1920.0, 1080.0),
];
/// Tamaño del contenido permitido (zoom de la página).
const ZOOMS: &[(f64, &str)] = &[(0.9, "Pequeño · 90 %"), (1.0, "Normal · 100 %"), (1.1, "Grande · 110 %")];
/// Espacio mínimo (a 100 %) para que el panel use su diseño de computadora (en el panel: > 1100 px de ancho y ≥ 640 de alto).
const ANCHO_PC: f64 = 1120.0;
const ALTO_PC: f64 = 660.0;

fn base() -> Value {
    serde_json::from_str(BASE).unwrap_or_else(|_| json!({}))
}

#[cfg(desktop)]
fn ruta_config(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

fn texto(c: &Value, k: &str, def: &str) -> String {
    c.get(k).and_then(|v| v.as_str()).unwrap_or(def).to_string()
}

#[cfg(desktop)]
fn tamano_valido(t: &str) -> bool {
    t == "maximizada" || TAMANOS.iter().any(|x| x.0 == t)
}

#[cfg(desktop)]
fn zoom_valido(z: f64) -> Option<f64> {
    ZOOMS.iter().map(|x| x.0).find(|v| (v - z).abs() < 0.001)
}

/// Tamaño de fábrica: el que se eligió al generar la app en el cPanel.
#[cfg(desktop)]
fn tamano_fabrica() -> String {
    let b = base();
    let t = texto(&b, "tamano", "");
    if tamano_valido(&t) {
        return t;
    }
    if b.get("maximizada").and_then(|v| v.as_bool()).unwrap_or(false) {
        "maximizada".into()
    } else {
        "compacta".into()
    }
}

/// Preferencias de esta computadora (tamano y zoom), ya revisadas. Cualquier otra cosa del archivo se ignora
/// (las versiones viejas guardaban ahí el servidor: ya no se usa).
#[cfg(desktop)]
fn preferencias(app: &AppHandle) -> (String, f64) {
    let mut tamano = tamano_fabrica();
    let mut zoom = 1.0;
    if let Some(p) = ruta_config(app) {
        if let Ok(t) = fs::read_to_string(p) {
            if let Ok(v) = serde_json::from_str::<Value>(&t) {
                let tt = texto(&v, "tamano", "");
                if tamano_valido(&tt) {
                    tamano = tt;
                }
                if let Some(z) = v.get("zoom").and_then(|x| x.as_f64()).and_then(zoom_valido) {
                    zoom = z;
                }
            }
        }
    }
    (tamano, zoom)
}

#[cfg(desktop)]
fn guardar_preferencias(app: &AppHandle, tamano: &str, zoom: f64) -> Result<(), String> {
    let ruta = ruta_config(app).ok_or_else(|| "No se encontró la carpeta de la app".to_string())?;
    if let Some(carpeta) = ruta.parent() {
        fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    }
    let datos = json!({ "tamano": tamano, "zoom": zoom });
    fs::write(ruta, serde_json::to_string_pretty(&datos).unwrap_or_default()).map_err(|e| e.to_string())
}

/// Pantalla en puntos lógicos (ancho, alto).
#[cfg(desktop)]
fn pantalla_app(app: &AppHandle) -> Option<(f64, f64)> {
    app.primary_monitor().ok().flatten().map(|m| {
        let s = m.size().to_logical::<f64>(m.scale_factor());
        (s.width, s.height)
    })
}

#[cfg(desktop)]
fn pantalla_ventana(w: &WebviewWindow) -> Option<(f64, f64)> {
    w.current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let s = m.size().to_logical::<f64>(m.scale_factor());
            (s.width, s.height)
        })
        .or_else(|| pantalla_app(w.app_handle()))
}

/// ¿Cabe la ventana en la pantalla? (se deja lugar para la barra de tareas / el Dock y la barra de título)
#[cfg(desktop)]
fn cabe(ancho: f64, alto: f64, pantalla: Option<(f64, f64)>) -> bool {
    match pantalla {
        Some((pw, ph)) => ancho <= pw - 20.0 && alto <= ph - 90.0,
        None => true,
    }
}

/// Medida que tendrá el contenido: (ancho, alto, maximizada). Si el tamaño no cabe en la pantalla, se maximiza.
#[cfg(desktop)]
fn medida(tamano: &str, pantalla: Option<(f64, f64)>) -> (f64, f64, bool) {
    if let Some(t) = TAMANOS.iter().find(|x| x.0 == tamano) {
        if cabe(t.2, t.3, pantalla) {
            return (t.2, t.3, false);
        }
    }
    match pantalla {
        Some((pw, ph)) => (pw, ph - 90.0, true),
        None => (1280.0, 800.0, true),
    }
}

/// ¿Con este zoom el panel sigue en su diseño de computadora?
#[cfg(desktop)]
fn zoom_cabe(z: f64, ancho: f64, alto: f64) -> bool {
    ancho / z >= ANCHO_PC && alto / z >= ALTO_PC
}

/// El zoom pedido, o el más grande de la lista que no rompa el diseño (si ninguno cabe, el más pequeño).
#[cfg(desktop)]
fn zoom_seguro(pedido: f64, ancho: f64, alto: f64) -> f64 {
    ZOOMS
        .iter()
        .map(|x| x.0)
        .filter(|z| *z <= pedido + 0.001 && zoom_cabe(*z, ancho, alto))
        .fold(None, |m: Option<f64>, z| Some(m.map_or(z, |a| a.max(z))))
        .unwrap_or(ZOOMS[0].0)
}

/// Android: una sola pantalla con el panel. Los enlaces de otros sitios (PayPal, WhatsApp, DGII…) van al navegador.
#[cfg(mobile)]
fn crear_ventana(app: &AppHandle, etiqueta: &str, url: WebviewUrl) -> tauri::Result<WebviewWindow> {
    let host = Url::parse(&texto(&base(), "servidor", ""))
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    let app_nav = app.clone();
    let init = INIT.replace("__VERSION__", &app.package_info().version.to_string());
    WebviewWindowBuilder::new(app, etiqueta, url)
        .initialization_script(&init)
        .on_navigation(move |u| {
            if es_interna(u, &host) {
                true
            } else {
                let _ = app_nav.opener().open_url(u.as_str(), None::<&str>);
                false
            }
        })
        .build()
}

#[cfg(desktop)]
fn crear_ventana(app: &AppHandle, etiqueta: &str, url: WebviewUrl) -> tauri::Result<WebviewWindow> {
    let c = base();
    let host = Url::parse(&texto(&c, "servidor", ""))
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    let app_nav = app.clone();
    let app_dl = app.clone();
    let init = INIT.replace("__VERSION__", &app.package_info().version.to_string());
    let (tamano, zoom) = preferencias(app);
    let pantalla = pantalla_app(app);
    let (ancho, alto, maximizada) = if etiqueta == "main" { medida(&tamano, pantalla) } else { medida("compacta", pantalla) };
    // Tamaño mínimo: el que necesita el panel para verse como en computadora (o lo que dé la pantalla si es más pequeña)
    let (min_w, min_h) = match pantalla {
        Some((pw, ph)) => (ANCHO_PC.min(pw * 0.9), ALTO_PC.min(ph * 0.8)),
        None => (900.0, 600.0),
    };
    let w = WebviewWindowBuilder::new(app, etiqueta, url)
        .title(texto(&c, "nombre", "Nido Gráfico"))
        .inner_size(ancho, alto)
        .min_inner_size(min_w, min_h)
        .center()
        .maximized(maximizada)
        .initialization_script(&init)
        .on_navigation(move |u| {
            if es_interna(u, &host) {
                true
            } else {
                let _ = app_nav.opener().open_url(u.as_str(), None::<&str>);
                false
            }
        })
        .on_download(move |webview, evento| {
            match evento {
                DownloadEvent::Requested { destination, .. } => {
                    if let Ok(carpeta) = app_dl.path().download_dir() {
                        let nombre = destination
                            .file_name()
                            .map(|n| n.to_os_string())
                            .unwrap_or_else(|| "descarga".into());
                        *destination = carpeta.join(nombre);
                    }
                }
                // 22-nov-2026 (pedido de Erick): al terminar, la página muestra dónde quedó el archivo (init.js → nidoDescargaLista)
                DownloadEvent::Finished { path, success, .. } => {
                    let ruta = path
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let js = format!(
                        "window.nidoDescargaLista && window.nidoDescargaLista({}, {})",
                        serde_json::to_string(&ruta).unwrap_or_else(|_| "\"\"".into()),
                        success
                    );
                    let _ = webview.eval(&js);
                }
                _ => {}
            }
            true
        })
        .build()?;
    let _ = w.set_zoom(zoom_seguro(zoom, ancho, alto));
    Ok(w)
}

/// ¿Se queda dentro de la app? Páginas propias de la app y el servidor (y sus subdominios).
fn es_interna(url: &Url, host: &str) -> bool {
    match url.scheme() {
        "tauri" | "data" | "blob" | "about" | "asset" => true,
        "http" | "https" => {
            let h = url.host_str().unwrap_or("");
            if h == "tauri.localhost" {
                return true;
            }
            !host.is_empty() && (h == host || h.ends_with(&format!(".{}", host)))
        }
        _ => false,
    }
}

#[cfg(desktop)]
fn ventana_principal(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main").ok_or_else(|| "No se encontró la ventana de la app".to_string())
}

/// Lo que muestra la ventana de configuración: tamaños y zooms con cuáles caben en esta pantalla.
#[cfg(desktop)]
fn estado(app: &AppHandle) -> Value {
    let (tamano, zoom) = preferencias(app);
    let pantalla = app.get_webview_window("main").and_then(|w| pantalla_ventana(&w)).or_else(|| pantalla_app(app));
    let (ancho, alto, _) = medida(&tamano, pantalla);
    let mut tamanos: Vec<Value> = TAMANOS
        .iter()
        .map(|t| json!({ "id": t.0, "nombre": t.1, "cabe": cabe(t.2, t.3, pantalla) }))
        .collect();
    tamanos.push(json!({ "id": "maximizada", "nombre": "Maximizada · toda la pantalla", "cabe": true }));
    let zooms: Vec<Value> = ZOOMS
        .iter()
        .map(|z| json!({ "valor": z.0, "nombre": z.1, "cabe": zoom_cabe(z.0, ancho, alto) }))
        .collect();
    json!({
        "tamano": tamano,
        "zoom": zoom_seguro(zoom, ancho, alto),
        "tamano_fabrica": tamano_fabrica(),
        "guardado": ruta_config(app).map(|p| p.exists()).unwrap_or(false),
        "tamanos": tamanos,
        "zooms": zooms,
        "version": app.package_info().version.to_string(),
        "nombre": texto(&base(), "nombre", "Nido Gráfico"),
    })
}

/// Pone el tamaño y el zoom en las ventanas abiertas (sin reiniciar la app).
#[cfg(desktop)]
fn poner(app: &AppHandle, tamano: &str, zoom: f64) -> Result<f64, String> {
    let w = ventana_principal(app)?;
    let pantalla = pantalla_ventana(&w);
    let (ancho, alto, maximizada) = medida(tamano, pantalla);
    if maximizada {
        w.maximize().map_err(|e| e.to_string())?;
    } else {
        let _ = w.unmaximize();
        w.set_size(LogicalSize::new(ancho, alto)).map_err(|e| e.to_string())?;
        let _ = w.center();
    }
    let z = zoom_seguro(zoom, ancho, alto);
    for (etiqueta, v) in app.webview_windows() {
        if etiqueta != "config" {
            let _ = v.set_zoom(z);
        }
    }
    Ok(z)
}

#[tauri::command]
fn leer_config(app: AppHandle) -> Value {
    // Lo que necesita la pantalla de carga (ui/index.html). El servidor sale siempre de app.json.
    let b = base();
    json!({
        "nombre": texto(&b, "nombre", "Nido Gráfico"),
        "servidor": texto(&b, "servidor", ""),
        "color_fondo": texto(&b, "color_fondo", ""),
        "color_texto": texto(&b, "color_texto", ""),
        "version": app.package_info().version.to_string(),
    })
}

#[cfg(desktop)]
#[tauri::command]
fn opciones(app: AppHandle) -> Value {
    estado(&app)
}

#[cfg(desktop)]
#[tauri::command]
fn aplicar(app: AppHandle, tamano: String, zoom: f64) -> Result<Value, String> {
    if !tamano_valido(&tamano) {
        return Err("Ese tamaño no está en la lista".into());
    }
    let z = zoom_valido(zoom).ok_or_else(|| "Ese tamaño de contenido no está en la lista".to_string())?;
    let puesto = poner(&app, &tamano, z)?;
    guardar_preferencias(&app, &tamano, puesto)?;
    Ok(estado(&app))
}

#[cfg(desktop)]
#[tauri::command]
fn restablecer(app: AppHandle) -> Result<Value, String> {
    if let Some(ruta) = ruta_config(&app) {
        let _ = fs::remove_file(ruta);
    }
    let (tamano, zoom) = preferencias(&app);
    poner(&app, &tamano, zoom)?;
    Ok(estado(&app))
}

// async: en Windows, crear una ventana desde un comando síncrono deja la app trabada
#[cfg(desktop)]
#[tauri::command]
async fn abrir_config(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("config") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return Ok(());
    }
    let (alto, min_alto) = match pantalla_app(&app) {
        Some((_, ph)) => ((ph - 120.0).min(720.0).max(480.0), (ph - 120.0).min(520.0).max(400.0)),
        None => (720.0, 520.0),
    };
    WebviewWindowBuilder::new(&app, "config", WebviewUrl::App("config.html".into()))
        .title("Configuración de la app")
        .inner_size(560.0, alto)
        .min_inner_size(460.0, min_alto)
        .center()
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Abre la configuración directo en «Asistente local» (lo llama el agente del panel y la primera vez que se abre la app)
#[cfg(desktop)]
#[tauri::command]
async fn abrir_asistente(app: AppHandle) -> Result<(), String> {
    ia::ENFOCAR_IA.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Some(w) = app.get_webview_window("config") {
        let _ = w.eval("window.iaEnfocar && window.iaEnfocar()");
    }
    abrir_config(app).await
}

#[cfg(desktop)]
#[tauri::command]
fn cerrar_config(app: AppHandle) {
    if let Some(w) = app.get_webview_window("config") {
        let _ = w.close();
    }
}

#[cfg(desktop)]
#[tauri::command]
fn recargar(app: AppHandle) -> Result<(), String> {
    ventana_principal(&app)?.eval("location.reload()").map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn ir_inicio(app: AppHandle) -> Result<(), String> {
    // Vuelve a la pantalla de carga, que conecta con el servidor de fábrica
    let js = "location.href = (window.NIDO_APP && window.NIDO_APP.mac ? 'tauri://localhost' : 'http://tauri.localhost') + '/index.html'";
    ventana_principal(&app)?.eval(js).map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn abrir_descargas(app: AppHandle) -> Result<(), String> {
    let carpeta = app.path().download_dir().map_err(|e| e.to_string())?;
    app.opener()
        .open_path(carpeta.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn imprimir(webview: tauri::Webview) -> Result<(), String> {
    webview.print().map_err(|e| e.to_string())
}

/// Android: los enlaces del panel se abren en la misma pantalla (init.js lo hace al recibir este error);
/// los de otros sitios, en el navegador del teléfono.
#[cfg(mobile)]
#[tauri::command]
async fn abrir_enlace(app: AppHandle, url: String) -> Result<(), String> {
    let u = Url::parse(&url).map_err(|e| e.to_string())?;
    let host = Url::parse(&texto(&base(), "servidor", ""))
        .ok()
        .and_then(|x| x.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    if es_interna(&u, &host) {
        return Err("misma pantalla".into());
    }
    app.opener().open_url(u.as_str(), None::<&str>).map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
async fn abrir_enlace(app: AppHandle, url: String) -> Result<(), String> {
    let u = Url::parse(&url).map_err(|e| e.to_string())?;
    let host = Url::parse(&texto(&base(), "servidor", ""))
        .ok()
        .and_then(|x| x.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    if es_interna(&u, &host) {
        let etiqueta = format!(
            "v-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        crear_ventana(&app, &etiqueta, WebviewUrl::External(u)).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        app.opener().open_url(u.as_str(), None::<&str>).map_err(|e| e.to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    #[cfg(desktop)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        leer_config,
        opciones,
        aplicar,
        restablecer,
        abrir_config,
        cerrar_config,
        recargar,
        ir_inicio,
        abrir_descargas,
        imprimir,
        abrir_enlace,
        abrir_asistente,
        ia::ia_estado,
        ia::ia_chat,
        ia::ia_guardar,
        ia::ia_descargar,
        ia::ia_instalar_ollama
    ]);
    #[cfg(mobile)]
    let builder = builder.invoke_handler(tauri::generate_handler![leer_config, abrir_enlace]);
    builder
        .setup(|app| {
            crear_ventana(app.handle(), "main", WebviewUrl::App("index.html".into()))?;
            // Primera vez en esta computadora: a los pocos segundos se abre el asistente local para instalarlo (una sola vez)
            #[cfg(desktop)]
            {
                let h = app.handle().clone();
                if !ia::existe_config(&h) {
                    let _ = ia::guardar_json(&h, &serde_json::json!({ "modelo": "", "activo": false, "visto": true }));
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(6));
                        let _ = tauri::async_runtime::block_on(abrir_asistente(h));
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar la app");
}
