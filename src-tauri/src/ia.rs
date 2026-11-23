// Asistente LOCAL (22-nov-2026, pedido de Erick: IA gratis, sin OpenAI). Solo computadora (Windows y Mac).
// El modelo corre en la computadora del usuario con Ollama (http://127.0.0.1:11434). Esta parte:
//  - ia_estado: ¿está Ollama? ¿qué modelos hay? ¿cuál eligió la persona y si está activo?
//  - ia_chat: le pasa una conversación al modelo (lo usa el agente del panel; las consultas a la base las hace el servidor).
//  - ia_descargar: baja el modelo elegido (con progreso en la ventana de configuración).
//  - ia_instalar_ollama: Windows baja el instalador oficial y lo abre; Mac abre la página oficial de descarga.
//  - ia_guardar: modelo elegido y encendido / apagado (ia.json en la carpeta de la app de esta computadora).
// Solo se aceptan los modelos de la lista MODELOS. Ollama escucha solo en esta computadora.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const OLLAMA: &str = "http://127.0.0.1:11434";

/// (id en Ollama, nombre, detalle). Qwen 2.5: bueno en español y sabe usar herramientas.
pub const MODELOS: &[(&str, &str, &str)] = &[
    ("qwen2.5:1.5b", "Ligero", "Unos 1 GB · computadoras sencillas (8 GB de RAM)"),
    ("qwen2.5:3b", "Recomendado", "Unos 2 GB · la mayoría de las computadoras"),
    ("qwen2.5:7b", "Avanzado", "Unos 4,7 GB · 16 GB de RAM o tarjeta de video"),
];

/// La ventana de configuración se abre directo en la sección del asistente (primera vez o desde el agente)
pub static ENFOCAR_IA: AtomicBool = AtomicBool::new(false);

fn ruta(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("ia.json"))
}

pub fn existe_config(app: &AppHandle) -> bool {
    ruta(app).map(|p| p.exists()).unwrap_or(false)
}

fn leer(app: &AppHandle) -> Value {
    ruta(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}))
}

pub fn guardar_json(app: &AppHandle, v: &Value) -> Result<(), String> {
    let p = ruta(app).ok_or_else(|| "No se encontró la carpeta de la app".to_string())?;
    if let Some(c) = p.parent() {
        fs::create_dir_all(c).map_err(|e| e.to_string())?;
    }
    fs::write(p, serde_json::to_string_pretty(v).unwrap_or_default()).map_err(|e| e.to_string())
}

fn cliente() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(3)).build()
}

/// Modelos instalados en Ollama. None = Ollama no está o no responde.
fn instalados() -> Option<Vec<String>> {
    let r = cliente().get(&format!("{}/api/tags", OLLAMA)).timeout(Duration::from_secs(4)).call().ok()?;
    let v: Value = r.into_json().ok()?;
    Some(
        v.get("models")?
            .as_array()?
            .iter()
            .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect(),
    )
}

fn texto(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

#[tauri::command]
pub async fn ia_estado(app: AppHandle) -> Value {
    let cfg = leer(&app);
    let lista = tauri::async_runtime::spawn_blocking(instalados).await.ok().flatten();
    let modelo = texto(&cfg, "modelo");
    let activo = cfg.get("activo").and_then(|v| v.as_bool()).unwrap_or(false);
    let ollama = lista.is_some();
    let lista = lista.unwrap_or_default();
    let tiene = !modelo.is_empty() && lista.iter().any(|n| n == &modelo);
    json!({
        "ollama": ollama,
        "instalados": lista,
        "modelo": modelo,
        "activo": activo,
        "listo": activo && ollama && tiene,
        "modelos": MODELOS.iter().map(|m| json!({ "id": m.0, "nombre": m.1, "detalle": m.2 })).collect::<Vec<_>>(),
        "so": std::env::consts::OS,
        "enfocar": ENFOCAR_IA.swap(false, Ordering::SeqCst),
    })
}

/// Conversación con el modelo local (sin flujo: espera la respuesta completa). El cuerpo es el de /api/chat de Ollama.
#[tauri::command]
pub async fn ia_chat(cuerpo: Value) -> Result<Value, String> {
    let mut c = cuerpo;
    match c.as_object_mut() {
        Some(o) => {
            o.insert("stream".into(), Value::Bool(false));
        }
        None => return Err("Petición inválida".into()),
    }
    tauri::async_runtime::spawn_blocking(move || {
        let r = cliente()
            .post(&format!("{}/api/chat", OLLAMA))
            .timeout(Duration::from_secs(300))
            .send_json(c)
            .map_err(|e| format!("El asistente local no respondió: {}", e))?;
        r.into_json::<Value>().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn ia_guardar(app: AppHandle, modelo: String, activo: bool) -> Result<Value, String> {
    if !modelo.is_empty() && !MODELOS.iter().any(|m| m.0 == modelo) {
        return Err("Ese modelo no está en la lista".into());
    }
    guardar_json(&app, &json!({ "modelo": modelo, "activo": activo, "visto": true }))?;
    Ok(json!({ "modelo": modelo, "activo": activo }))
}

/// Avisa el progreso a la ventana de configuración (window.iaProgreso)
fn avisar(app: &AppHandle, v: Value) {
    if let Some(w) = app.get_webview_window("config") {
        let _ = w.eval(&format!("window.iaProgreso && window.iaProgreso({})", v));
    }
}

/// Baja un modelo de la lista con Ollama, avisando el progreso.
#[tauri::command]
pub async fn ia_descargar(app: AppHandle, modelo: String) -> Result<(), String> {
    if !MODELOS.iter().any(|m| m.0 == modelo) {
        return Err("Ese modelo no está en la lista".into());
    }
    std::thread::spawn(move || {
        let r = cliente().post(&format!("{}/api/pull", OLLAMA)).send_json(json!({ "name": modelo, "stream": true }));
        let resp = match r {
            Ok(x) => x,
            Err(e) => {
                avisar(&app, json!({ "fin": true, "ok": false, "error": format!("No se pudo descargar: {}", e) }));
                return;
            }
        };
        let mut ok = false;
        let mut ultimo = Instant::now() - Duration::from_secs(1);
        for linea in BufReader::new(resp.into_reader()).lines().flatten() {
            if let Ok(v) = serde_json::from_str::<Value>(&linea) {
                if let Some(err) = v.get("error") {
                    avisar(&app, json!({ "fin": true, "ok": false, "error": err.clone() }));
                    return;
                }
                if texto(&v, "status") == "success" {
                    ok = true;
                }
                if ultimo.elapsed() >= Duration::from_millis(300) {
                    ultimo = Instant::now();
                    avisar(&app, json!({ "estado": texto(&v, "status"), "hecho": v.get("completed").cloned().unwrap_or(Value::Null), "total": v.get("total").cloned().unwrap_or(Value::Null) }));
                }
            }
        }
        avisar(&app, json!({ "fin": true, "ok": ok }));
    });
    Ok(())
}

/// Windows: baja el instalador oficial de Ollama y lo abre. Mac: abre la página oficial (se instala arrastrándolo a Aplicaciones).
#[tauri::command]
pub async fn ia_instalar_ollama(app: AppHandle) -> Result<String, String> {
    if !cfg!(target_os = "windows") {
        app.opener().open_url("https://ollama.com/download", None::<&str>).map_err(|e| e.to_string())?;
        return Ok("navegador".into());
    }
    std::thread::spawn(move || {
        let destino = std::env::temp_dir().join("OllamaSetup.exe");
        let resp = match cliente().get("https://ollama.com/download/OllamaSetup.exe").call() {
            Ok(x) => x,
            Err(e) => {
                avisar(&app, json!({ "instalador": true, "fin": true, "ok": false, "error": format!("No se pudo bajar el instalador: {}", e) }));
                return;
            }
        };
        let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
        let mut lector = resp.into_reader();
        let mut archivo = match fs::File::create(&destino) {
            Ok(f) => f,
            Err(e) => {
                avisar(&app, json!({ "instalador": true, "fin": true, "ok": false, "error": e.to_string() }));
                return;
            }
        };
        let mut buf = vec![0u8; 256 * 1024];
        let mut hecho: u64 = 0;
        let mut ultimo = Instant::now() - Duration::from_secs(1);
        loop {
            match lector.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if archivo.write_all(&buf[..n]).is_err() {
                        avisar(&app, json!({ "instalador": true, "fin": true, "ok": false, "error": "No se pudo guardar el instalador" }));
                        return;
                    }
                    hecho += n as u64;
                    if ultimo.elapsed() >= Duration::from_millis(300) {
                        ultimo = Instant::now();
                        avisar(&app, json!({ "instalador": true, "estado": "Bajando Ollama", "hecho": hecho, "total": total }));
                    }
                }
                Err(e) => {
                    avisar(&app, json!({ "instalador": true, "fin": true, "ok": false, "error": e.to_string() }));
                    return;
                }
            }
        }
        drop(archivo);
        let ok = std::process::Command::new(&destino).spawn().is_ok();
        avisar(&app, json!({ "instalador": true, "fin": true, "ok": ok, "abierto": ok }));
    });
    Ok("descargando".into())
}
