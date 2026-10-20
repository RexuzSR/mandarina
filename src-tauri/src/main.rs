#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING, MB_OK};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Manager};

const IMG: &[&str] = &["png", "jpg", "webp", "bmp", "gif", "tiff", "ico"];
const AUD: &[&str] = &["mp3", "wav", "flac", "ogg", "m4a", "aac"];
const VID: &[&str] = &["mp4", "mov", "mkv", "webm", "avi"];
const OFFICE: &[&str] = &["docx", "doc", "odt", "rtf"];
const TEXTDOC: &[&str] = &["txt", "html"];
const MD: &[&str] = &["md"];
const PDF: &[&str] = &["pdf"];

fn ext(p: &str) -> String {
    let e = Path::new(p).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match e.as_str() { "jpeg" => "jpg".into(), "tif" => "tiff".into(), "htm" => "html".into(), "markdown" => "md".into(), _ => e }
}

/// DIAGNÓSTICO: registro en %TEMP%\mandarina.log
fn log(msg: &str) {
    use std::io::Write;
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let p = std::env::temp_dir().join("mandarina.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) { let _ = writeln!(f, "{} {}", t % 1_000_000, msg); }
}
#[tauri::command]
fn log_js(msg: String) { log(&format!("[js] {msg}")); }

/// Formatos a los que puede convertirse el archivo (según su extensión).
#[tauri::command]
fn formats_for(path: String) -> Vec<String> {
    let e = ext(&path);
    let list: &[&str] = if IMG.contains(&e.as_str()) { &["png", "jpg", "webp", "bmp", "gif", "tiff", "pdf"] }
        else if OFFICE.contains(&e.as_str()) { &["pdf", "docx", "odt", "txt", "html"] }
        else if TEXTDOC.contains(&e.as_str()) { &["pdf", "docx", "odt"] }
        else if MD.contains(&e.as_str()) { &["docx", "html", "txt", "epub"] }
        else if PDF.contains(&e.as_str()) { &["png", "jpg", "txt"] }
        else if AUD.contains(&e.as_str()) { &["mp3", "wav", "flac", "ogg", "m4a", "aac"] }
        else if VID.contains(&e.as_str()) { &["mp4", "webm", "mkv", "mov", "gif", "mp3"] }
        else { &[] };
    list.iter().filter(|f| **f != e).map(|f| f.to_string()).collect()
}

fn out_path(src: &Path, target: &str) -> PathBuf {
    let dir = src.parent().unwrap_or(Path::new("."));
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("salida");
    let mut p = dir.join(format!("{stem}.{target}"));
    let mut i = 1;
    while p.exists() { p = dir.join(format!("{stem} ({i}).{target}")); i += 1; }
    p
}

fn ffmpeg(src: &Path, out: &Path) -> Result<(), String> {
    let mut c = std::process::Command::new("ffmpeg");
    c.args(["-y", "-i"]).arg(src).arg(out);
    #[cfg(windows)]
    { use std::os::windows::process::CommandExt; c.creation_flags(0x08000000); }
    let o = c.output().map_err(|_| "Falta ffmpeg para convertir audio y video. Instálalo (winget install Gyan.FFmpeg) y vuelve a intentarlo.".to_string())?;
    if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).into()) }
}

fn run(c: &mut std::process::Command, name: &str) -> Result<(), String> {
    #[cfg(windows)]
    { use std::os::windows::process::CommandExt; c.creation_flags(0x08000000); }
    let o = c.output().map_err(|_| format!("Falta {name}. Instálalo y asegúrate de que esté en el PATH (mira el README) y vuelve a intentarlo."))?;
    if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).into()) }
}

/// Word/ODT/RTF/TXT/HTML con LibreOffice (sin interfaz). Convierte en una carpeta temporal y mueve el resultado.
fn soffice(src: &Path, target: &str, out: &Path) -> Result<(), String> {
    let bin = [r"C:\Program Files\LibreOffice\program\soffice.exe", r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"]
        .iter().find(|p| Path::new(p).exists()).map(|p| p.to_string()).unwrap_or("soffice".into());
    let tmp = std::env::temp_dir().join(format!("mandarina-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let filter = match target { "docx" => "docx:MS Word 2007 XML", "txt" => "txt:Text", t => t };
    let profile = format!("-env:UserInstallation=file:///{}", tmp.join("perfil").to_string_lossy().replace('\\', "/"));
    let mut c = std::process::Command::new(bin);
    c.arg(profile).args(["--headless", "--convert-to", filter, "--outdir"]).arg(&tmp);
    if ext(&src.to_string_lossy()) == "html" { c.arg("--infilter=HTML (StarWriter)"); }
    c.arg(src);
    let r = run(&mut c, "LibreOffice");
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("salida");
    let made = tmp.join(format!("{stem}.{target}"));
    let r = r.and_then(|_| std::fs::copy(&made, out).map(|_| ()).map_err(|e| e.to_string()));
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

/// Markdown con pandoc.
fn pandoc(src: &Path, target: &str, out: &Path) -> Result<(), String> {
    let mut c = std::process::Command::new("pandoc");
    c.arg(src).arg("-s");
    if target == "txt" { c.args(["-t", "plain"]); }
    c.arg("-o").arg(out);
    run(&mut c, "pandoc")
}

/// PDF a texto o a imágenes (una por página, dentro de una carpeta nueva) con poppler.
fn pdf_tool(src: &Path, target: &str, out: &Path) -> Result<PathBuf, String> {
    if target == "txt" {
        let mut c = std::process::Command::new("pdftotext");
        c.arg("-layout").arg(src).arg(out);
        run(&mut c, "pdftotext (poppler)")?;
        return Ok(out.to_path_buf());
    }
    let parent = src.parent().unwrap_or(Path::new("."));
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("salida");
    let mut dir = parent.join(format!("{stem} (paginas)"));
    let mut i = 1;
    while dir.exists() { dir = parent.join(format!("{stem} (paginas {i})")); i += 1; }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut c = std::process::Command::new("pdftoppm");
    c.args(["-r", "150", if target == "jpg" { "-jpeg" } else { "-png" }]).arg(src).arg(dir.join("pagina"));
    run(&mut c, "pdftoppm (poppler)")?;
    Ok(dir)
}

/// Imagen a PDF de una página, sin dependencias externas (JPEG incrustado).
fn img_to_pdf(src: &Path, out: &Path) -> Result<(), String> {
    let img = image::open(src).map_err(|e| e.to_string())?.to_rgb8();
    let (w, h) = img.dimensions();
    let mut jpg = vec![];
    image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut jpg), image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    let (pw, ph) = (w as f32 * 72.0 / 150.0, h as f32 * 72.0 / 150.0);
    let content = format!("q {pw:.1} 0 0 {ph:.1} 0 0 cm /Im0 Do Q");
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offs = vec![];
    offs.push(pdf.len()); pdf.extend(b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n");
    offs.push(pdf.len()); pdf.extend(b"2 0 obj\n<</Type/Pages/Kids[3 0 R]/Count 1>>\nendobj\n");
    offs.push(pdf.len()); pdf.extend(format!("3 0 obj\n<</Type/Page/Parent 2 0 R/MediaBox[0 0 {pw:.1} {ph:.1}]/Resources<</XObject<</Im0 4 0 R>>>>/Contents 5 0 R>>\nendobj\n").as_bytes());
    offs.push(pdf.len());
    pdf.extend(format!("4 0 obj\n<</Type/XObject/Subtype/Image/Width {w}/Height {h}/ColorSpace/DeviceRGB/BitsPerComponent 8/Filter/DCTDecode/Length {}>>\nstream\n", jpg.len()).as_bytes());
    pdf.extend(&jpg); pdf.extend(b"\nendstream\nendobj\n");
    offs.push(pdf.len()); pdf.extend(format!("5 0 obj\n<</Length {}>>\nstream\n{content}\nendstream\nendobj\n", content.len()).as_bytes());
    let xref = pdf.len();
    pdf.extend(b"xref\n0 6\n0000000000 65535 f \n");
    for o in &offs { pdf.extend(format!("{o:010} 00000 n \n").as_bytes()); }
    pdf.extend(format!("trailer\n<</Size 6/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF").as_bytes());
    std::fs::write(out, pdf).map_err(|e| e.to_string())
}

/// Convierte cada archivo y deja el resultado junto al original.
fn convert_inner(paths: Vec<String>, target: String) -> Result<Vec<String>, String> {
    let mut done = vec![];
    for s in paths {
        let src = Path::new(&s);
        let out = out_path(src, &target);
        let e = ext(&s);
        let result: PathBuf = if IMG.contains(&e.as_str()) && target == "pdf" {
            img_to_pdf(src, &out)?; out
        } else if IMG.contains(&e.as_str()) {
            let img = image::open(src).map_err(|e| e.to_string())?;
            let img = if target == "jpg" || target == "bmp" { image::DynamicImage::ImageRgb8(img.to_rgb8()) } else { img };
            img.save(&out).map_err(|e| e.to_string())?; out
        } else if OFFICE.contains(&e.as_str()) || TEXTDOC.contains(&e.as_str()) {
            soffice(src, &target, &out)?; out
        } else if MD.contains(&e.as_str()) {
            pandoc(src, &target, &out)?; out
        } else if PDF.contains(&e.as_str()) {
            pdf_tool(src, &target, &out)?
        } else {
            ffmpeg(src, &out)?; out
        };
        done.push(result.to_string_lossy().to_string());
    }
    Ok(done)
}

/// Muestra un aviso nativo de Windows (en otro hilo para no bloquear).
#[cfg(windows)]
fn aviso(msg: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    let chars: Vec<char> = msg.chars().collect();
    let msg: String = chars[chars.len().saturating_sub(600)..].iter().collect();
    let t: Vec<u16> = "Mandarina\0".encode_utf16().collect();
    let m: Vec<u16> = msg.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { MessageBoxW(HWND::default(), PCWSTR(m.as_ptr()), PCWSTR(t.as_ptr()), MB_OK | MB_ICONWARNING); }
}
#[cfg(not(windows))]
fn aviso(_: &str) {}

#[tauri::command]
async fn convert(paths: Vec<String>, target: String) -> Result<Vec<String>, String> {
    let r = convert_inner(paths, target);
    if let Err(e) = &r { let m = e.clone(); std::thread::spawn(move || aviso(&m)); }
    r
}

/// Vigila: botón izquierdo + Shift y movimiento > 12 px => muestra el overlay que recibe el arrastre.
#[cfg(windows)]
fn watcher(app: tauri::AppHandle) {
    use std::{thread::sleep, time::Duration};
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    let down = |vk: VIRTUAL_KEY| unsafe { (GetAsyncKeyState(vk.0 as i32) as u16 & 0x8000) != 0 };
    let cursor = || { let mut p = POINT::default(); unsafe { let _ = GetCursorPos(&mut p); } (p.x, p.y) };
    let mut start: Option<(i32, i32)> = None;
    let mut shown = false;
    log("watcher iniciado");

    loop {
        sleep(Duration::from_millis(15));
        let Some(w) = app.get_webview_window("overlay") else { continue };
        if !down(VK_LBUTTON) {
            if shown { sleep(Duration::from_millis(150)); let _ = w.hide(); shown = false; log("overlay oculto"); }
            start = None;
            continue;
        }
        if shown { continue; }
        if !down(VK_SHIFT) { start = None; continue; }
        let (x, y) = cursor();
        match start {
            None => { start = Some((x, y)); log("Shift+clic detectado"); }
            Some((sx, sy)) if (x - sx).abs().max((y - sy).abs()) > 12 => {
                let (vx, vy, vw, vh) = unsafe { (
                    GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN),
                    GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) };
                let _ = w.set_position(tauri::PhysicalPosition::new(vx, vy));
                let _ = w.set_size(tauri::PhysicalSize::new(vw as u32, vh as u32));
                let r = w.show(); log(&format!("overlay show ok={} en {vx},{vy} {vw}x{vh}", r.is_ok()));
                shown = true;
            }
            _ => {}
        }
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![formats_for, convert, log_js])
        .setup(|app| {
            log("app iniciada");
            let quit = MenuItem::with_id(app, "quit", "Salir de Mandarina", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Mandarina: mantén Shift al arrastrar un archivo")
                .menu(&menu)
                .on_menu_event(|app, e| if e.id.as_ref() == "quit" { app.exit(0) })
                .build(app)?;
            #[cfg(windows)]
            { let h = app.handle().clone(); std::thread::spawn(move || watcher(h)); }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al iniciar Mandarina");
}
