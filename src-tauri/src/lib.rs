// App de escritorio de Nido Gráfico (Tauri 2). Generada por el módulo app/escritorio del panel.
// La ventana carga ui/index.html (pantalla de carga) y de ahí el servidor de app.json (o el que se guardó con el engranaje).
// - Enlaces a otros sitios (PayPal, WhatsApp, DGII…) se abren en el navegador normal.
// - Enlaces del mismo servidor con target=_blank / window.open abren otra ventana de la app.
// - Descargas van a la carpeta Descargas. En Mac, window.print() se pasa al diálogo nativo (WKWebView no imprime solo).
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use tauri::{webview::DownloadEvent, AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const BASE: &str = include_str!("../../app.json");
const INIT: &str = include_str!("init.js");

fn base() -> Value {
    serde_json::from_str(BASE).unwrap_or_else(|_| json!({}))
}

fn ruta_config(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

/// app.json + lo guardado en esta computadora con el engranaje.
fn config(app: &AppHandle) -> Value {
    let mut c = base();
    if let Some(p) = ruta_config(app) {
        if let Ok(texto) = fs::read_to_string(p) {
            if let Ok(Value::Object(propio)) = serde_json::from_str::<Value>(&texto) {
                if let Value::Object(ref mut m) = c {
                    for (k, v) in propio {
                        m.insert(k, v);
                    }
                }
            }
        }
    }
    c
}

fn texto(c: &Value, k: &str, def: &str) -> String {
    c.get(k).and_then(|v| v.as_str()).unwrap_or(def).to_string()
}

fn numero(c: &Value, k: &str, def: f64) -> f64 {
    c.get(k).and_then(|v| v.as_f64()).unwrap_or(def)
}

fn host_servidor(app: &AppHandle) -> String {
    Url::parse(&texto(&config(app), "servidor", ""))
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
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

fn crear_ventana(app: &AppHandle, etiqueta: &str, url: WebviewUrl) -> tauri::Result<tauri::WebviewWindow> {
    let c = config(app);
    let host = host_servidor(app);
    let app_nav = app.clone();
    let app_dl = app.clone();
    let init = INIT.replace("__VERSION__", &app.package_info().version.to_string());
    WebviewWindowBuilder::new(app, etiqueta, url)
        .title(texto(&c, "nombre", "Nido Gráfico"))
        .inner_size(numero(&c, "ancho", 1280.0), numero(&c, "alto", 800.0))
        .min_inner_size(900.0, 600.0)
        .maximized(c.get("maximizada").and_then(|v| v.as_bool()).unwrap_or(false))
        .initialization_script(&init)
        .on_navigation(move |u| {
            if es_interna(u, &host) {
                true
            } else {
                let _ = app_nav.opener().open_url(u.as_str(), None::<&str>);
                false
            }
        })
        .on_download(move |_webview, evento| {
            if let DownloadEvent::Requested { destination, .. } = evento {
                if let Ok(carpeta) = app_dl.path().download_dir() {
                    let nombre = destination
                        .file_name()
                        .map(|n| n.to_os_string())
                        .unwrap_or_else(|| "descarga".into());
                    *destination = carpeta.join(nombre);
                }
            }
            true
        })
        .build()
}

#[tauri::command]
fn leer_config(app: AppHandle) -> Value {
    let mut c = config(&app);
    if let Value::Object(ref mut m) = c {
        m.remove("pin_hash");
        m.insert("version".into(), json!(app.package_info().version.to_string()));
        m.insert("servidor_original".into(), json!(texto(&base(), "servidor", "")));
    }
    c
}

#[tauri::command]
fn verificar_pin(pin: String) -> bool {
    let guardado = texto(&base(), "pin_hash", "");
    !guardado.is_empty() && format!("{:x}", Sha256::digest(pin.trim().as_bytes())) == guardado
}

#[tauri::command]
fn guardar_config(app: AppHandle, pin: String, datos: Value) -> Result<(), String> {
    if !verificar_pin(pin) {
        return Err("PIN incorrecto".into());
    }
    let ruta = ruta_config(&app).ok_or_else(|| "No se encontró la carpeta de la app".to_string())?;
    if let Some(carpeta) = ruta.parent() {
        fs::create_dir_all(carpeta).map_err(|e| e.to_string())?;
    }
    let mut limpio = serde_json::Map::new();
    if let Value::Object(m) = datos {
        for k in ["servidor", "ancho", "alto", "maximizada"] {
            if let Some(v) = m.get(k) {
                limpio.insert(k.to_string(), v.clone());
            }
        }
    }
    fs::write(ruta, serde_json::to_string_pretty(&Value::Object(limpio)).unwrap_or_default()).map_err(|e| e.to_string())
}

#[tauri::command]
fn restablecer_config(app: AppHandle, pin: String) -> Result<(), String> {
    if !verificar_pin(pin) {
        return Err("PIN incorrecto".into());
    }
    if let Some(ruta) = ruta_config(&app) {
        let _ = fs::remove_file(ruta);
    }
    Ok(())
}

#[tauri::command]
fn reiniciar(app: AppHandle) {
    app.restart();
}

#[tauri::command]
fn imprimir(webview: tauri::Webview) -> Result<(), String> {
    webview.print().map_err(|e| e.to_string())
}

#[tauri::command]
async fn abrir_enlace(app: AppHandle, url: String) -> Result<(), String> {
    let u = Url::parse(&url).map_err(|e| e.to_string())?;
    if es_interna(&u, &host_servidor(&app)) {
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            leer_config,
            verificar_pin,
            guardar_config,
            restablecer_config,
            reiniciar,
            imprimir,
            abrir_enlace
        ])
        .setup(|app| {
            crear_ventana(app.handle(), "main", WebviewUrl::App("index.html".into()))?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("No se pudo iniciar la app");
}
