# 🍊 Mandarina

Mantén **Shift** mientras arrastras un archivo y aparece un menú alrededor del cursor con los formatos a los que puede convertirse. Suelta sobre uno y el archivo nuevo queda junto al original. Todo ocurre en tu equipo: nada se sube a internet.

## Descargar (Windows 10/11)

**[⬇ Descargar la última versión](../../releases/latest)**

- `Mandarina_..._x64-setup.exe`: instalador.
- `Mandarina-portable.zip`: sin instalar; descomprime y ejecuta `mandarina.exe`.

> **Aviso de Windows:** al abrirlo puede aparecer "Windows protegió su PC". Es normal en programas nuevos sin firma de pago. Pulsa **Más información > Ejecutar de todas formas**.

## Usar
1. Abre Mandarina: queda en la bandeja del sistema (icono de mandarina; clic derecho > Salir).
2. Arrastra un archivo desde el Explorador **manteniendo Shift**.
3. Mueve el cursor hacia un formato y suelta.

## ¿Qué convierte?
| Archivo | A | Necesita |
|---|---|---|
| Imágenes (PNG, JPG, WEBP, BMP, GIF, TIFF, ICO) | otras imágenes y PDF | nada |
| Audio y video | otros formatos de audio/video | [ffmpeg](https://ffmpeg.org) |
| Word, ODT, RTF, TXT, HTML | PDF, DOCX, ODT, TXT, HTML | [LibreOffice](https://www.libreoffice.org) |
| Markdown | DOCX, HTML, TXT, EPUB | [pandoc](https://pandoc.org) |
| PDF | imágenes y texto | [poppler](https://poppler.freedesktop.org) |

Mandarina **no instala ni cambia nada**: usa los programas que ya tengas. Si falta alguno, te avisa cuál. Con winget:
`winget install Gyan.FFmpeg`, `winget install TheDocumentFoundation.LibreOffice`, `winget install JohnMacFarlane.Pandoc`; poppler: `scoop install poppler`.

## Para desarrolladores
Requiere Rust, Node.js y Visual Studio Build Tools (C++).

    npm install
    npm run dev       # desarrollo
    npm run build     # instalador en src-tauri/target/release/bundle/nsis

## Publicar una versión
En GitHub: **Releases > Draft a new release**, crea la etiqueta (por ejemplo `v0.1.0`) y publica. GitHub Actions compila y adjunta el instalador y el zip portable.
