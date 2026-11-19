// 19-nov-2026: lista de comandos propios de la app. Con esta lista Tauri crea un permiso «allow-<comando>» por cada uno, y
// el cPanel los pone en capabilities/default.json (escritorio-helper.php). Sin esto, las páginas del servidor (el panel)
// recibían «Command opciones not allowed by ACL» al abrir el engranaje. Si se agrega un comando en src/lib.rs, va aquí también.
const COMANDOS: &[&str] = &[
    "leer_config",
    "opciones",
    "aplicar",
    "restablecer",
    "abrir_config",
    "cerrar_config",
    "recargar",
    "ir_inicio",
    "abrir_descargas",
    "imprimir",
    "abrir_enlace",
];

fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMANDOS)))
        .expect("No se pudo preparar la app (tauri-build)");
}
