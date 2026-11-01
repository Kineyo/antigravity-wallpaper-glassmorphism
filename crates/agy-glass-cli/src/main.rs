use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::process::Command;
use tempfile::Builder as TempDirBuilder;

use agy_glass_core::{
    find_antigravity_asar, get_candidate_asar_paths, get_wallpaper_dirs,
    get_custom_css_path, get_backup_base_dir, get_devtools_port_file,
    create_backup, verify_backup, optimize_image_to_data_uri,
    generate_css, GlassConfig, patch_preload, patch_utils,
};

#[derive(Parser)]
#[command(name = "agy-glass")]
#[command(author = "Kineyo & DarwinSin")]
#[command(version = "0.2.0")]
#[command(about = "Motor nativo ultrarrápido de personalización Glassmorphism para Antigravity IDE", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parchea Antigravity IDE instalando el motor reactivo de persistencia
    Patch {
        #[arg(short, long, help = "Ruta manual al archivo resources/app.asar")]
        asar: Option<PathBuf>,
    },

    /// Aplica un wallpaper con efecto Glassmorphism y capa scrim de contraste
    Apply {
        #[arg(help = "Ruta o nombre del archivo de imagen (JPG, PNG, WebP)")]
        image: String,

        #[arg(long, default_value_t = 0.65, help = "Opacidad de fondo (0.0 a 1.0)")]
        opacity: f32,

        #[arg(long, default_value_t = 12, help = "Intensidad del desenfoque blur en píxeles")]
        blur: u32,

        #[arg(long, default_value_t = 0.45, help = "Opacidad de la capa scrim de contraste (0.0 a 1.0)")]
        scrim: f32,
    },

    /// Lista los wallpapers disponibles en las carpetas detectadas (XDG Pictures)
    List,

    /// Restaura Antigravity IDE a su estado previo sin modificaciones
    Restore {
        #[arg(short, long, help = "Ruta manual al archivo resources/app.asar")]
        asar: Option<PathBuf>,

        #[arg(long, help = "Restaura el respaldo más reciente en lugar del de fábrica")]
        latest: bool,
    },

    /// Diagnostica el estado del sistema, rutas detectadas y configuración activa
    Doctor,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Patch { asar } => {
            println!("{}", "🌌 Iniciando parcheo de Antigravity IDE...".bold().cyan());
            let asar_path = match find_antigravity_asar(asar.as_deref()) {
                Some(p) => p,
                None => {
                    eprintln!("{}", "❌ Error: No se encontró app.asar de Antigravity.".bold().red());
                    eprintln!("🔍 Rutas inspeccionadas:");
                    for cand in get_candidate_asar_paths() {
                        eprintln!("  • {}", cand.display());
                    }
                    eprintln!("\n💡 Puedes especificar la ruta con: agy-glass patch --asar \"/ruta/a/app.asar\"");
                    std::process::exit(1);
                }
            };

            println!("{} {}", "🔍 Antigravity detectado en:".green(), asar_path.display());
            let backup_base = get_backup_base_dir();
            println!("{}", "📦 Creando copia de seguridad verificada con SHA-256...".yellow());
            let (backup_file, sha) = create_backup(&asar_path, &backup_base)?;
            println!("{} {} (SHA-256: {})", "✅ Respaldo creado en:".green(), backup_file.display(), &sha[..16]);

            let tmp_dir = TempDirBuilder::new().prefix("agy_glass_").tempdir()?;
            let extract_dir = tmp_dir.path().join("extracted");
            let repack_file = tmp_dir.path().join("repacked.asar");

            println!("{}", "📂 Extrayendo app.asar con @electron/asar...".cyan());
            let extract_status = Command::new("npx")
                .args(["--yes", "@electron/asar", "extract", asar_path.to_str().unwrap(), extract_dir.to_str().unwrap()])
                .status()?;

            if !extract_status.success() {
                eprintln!("{}", "❌ Error al extraer app.asar".bold().red());
                std::process::exit(1);
            }

            let preload_path = extract_dir.join("dist/preload.js");
            let utils_path = extract_dir.join("dist/utils.js");

            if !preload_path.exists() || !utils_path.exists() {
                eprintln!("{}", "❌ Estructura de app.asar inesperada. Falta preload.js o utils.js".bold().red());
                std::process::exit(1);
            }

            println!("{}", "⚡ Inyectando motor reactivo en preload.js...".cyan());
            let preload_code = std::fs::read_to_string(&preload_path)?;
            let patched_preload = patch_preload(&preload_code);
            std::fs::write(&preload_path, patched_preload)?;

            println!("{}", "⚡ Parcheando utils.js (DevTools y anclaje Chromium)...".cyan());
            let utils_code = std::fs::read_to_string(&utils_path)?;
            if let Ok(patched_utils) = patch_utils(&utils_code) {
                std::fs::write(&utils_path, patched_utils)?;
            } else {
                println!("{}", "⚠️ Aviso: Punto de anclaje no encontrado en utils.js; se mantendrá inyección preload.".yellow());
            }

            println!("{}", "📦 Reempaquetando app.asar...".cyan());
            let pack_status = Command::new("npx")
                .args(["--yes", "@electron/asar", "pack", extract_dir.to_str().unwrap(), repack_file.to_str().unwrap()])
                .status()?;

            if !pack_status.success() {
                eprintln!("{}", "❌ Error al reempaquetar app.asar".bold().red());
                std::process::exit(1);
            }

            std::fs::copy(&repack_file, &asar_path)?;
            println!("{}", "🎉 ¡Antigravity IDE parcheado exitosamente con el motor nativo!".bold().green());
            println!("💡 Para aplicar un wallpaper ejecuta: agy-glass apply \"imagen.jpg\"");
        }

        Commands::Apply { image, opacity, blur, scrim } => {
            let mut image_path = PathBuf::from(&image);
            if !image_path.exists() {
                let mut found = false;
                for wdir in get_wallpaper_dirs() {
                    let alt = wdir.join(&image);
                    if alt.exists() {
                        image_path = alt;
                        found = true;
                        break;
                    }
                }
                if !found {
                    eprintln!("{} {}", "❌ Error: No se encontró la imagen:".bold().red(), image);
                    std::process::exit(1);
                }
            }

            let file_name = image_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            println!("{} {}", "🖼️ Procesando imagen:".cyan(), file_name);

            let data_uri = optimize_image_to_data_uri(&image_path)?;
            let config = GlassConfig {
                wallpaper_name: file_name,
                background_opacity: opacity,
                sidebar_opacity: opacity + 0.07,
                card_opacity: opacity + 0.05,
                scrim_opacity: scrim,
                sidebar_blur_px: blur,
                card_blur_px: blur.saturating_sub(2),
            };

            let css = generate_css(&data_uri, &config);
            let css_path = get_custom_css_path();
            if let Some(parent) = css_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&css_path, css)?;

            println!("{} {}", "✅ Estilo persistente guardado en:".green(), css_path.display());
            println!("{}", "✨ ¡Wallpaper Glassmorphism establecido con éxito!".bold().green());
        }

        Commands::List => {
            let dirs = get_wallpaper_dirs();
            if dirs.is_empty() {
                println!("{}", "ℹ️ No se encontraron carpetas de imágenes XDG.".yellow());
                return Ok(());
            }

            let mut count = 0;
            for d in dirs {
                if let Ok(entries) = std::fs::read_dir(&d) {
                    let mut files = Vec::new();
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            let ext_lower = ext.to_lowercase();
                            if ["jpg", "jpeg", "png", "webp"].contains(&ext_lower.as_str()) {
                                files.push(path.file_name().unwrap().to_string_lossy().to_string());
                            }
                        }
                    }
                    if !files.is_empty() {
                        files.sort();
                        println!("{} {}:", "📁 Carpeta:".cyan().bold(), d.display());
                        for f in files {
                            count += 1;
                            println!("  • {}", f);
                        }
                    }
                }
            }
            if count == 0 {
                println!("{}", "ℹ️ No se encontraron imágenes en las carpetas inspeccionadas.".yellow());
            }
        }

        Commands::Restore { asar, latest } => {
            println!("{}", "🔄 Iniciando restauración de Antigravity IDE...".bold().cyan());
            let asar_path = match find_antigravity_asar(asar.as_deref()) {
                Some(p) => p,
                None => {
                    eprintln!("{}", "❌ Error: No se encontró la ruta destino de app.asar.".bold().red());
                    std::process::exit(1);
                }
            };

            let backup_base = get_backup_base_dir();
            if !backup_base.exists() {
                eprintln!("{}", "❌ No se encontraron respaldos previos.".bold().red());
                std::process::exit(1);
            }

            let mut backups = Vec::new();
            for entry in std::fs::read_dir(&backup_base)?.flatten() {
                let candidate = entry.path().join("app.asar");
                if candidate.is_file() {
                    backups.push(candidate);
                }
            }

            if backups.is_empty() {
                eprintln!("{}", "❌ No se encontraron archivos de respaldo app.asar.".bold().red());
                std::process::exit(1);
            }

            backups.sort();
            let target_backup = if latest {
                backups.last().unwrap()
            } else {
                backups.first().unwrap()
            };

            println!("{} {}", "📦 Verificando respaldo:".cyan(), target_backup.display());
            verify_backup(target_backup)?;
            println!("{}", "✅ Verificación criptográfica SHA-256: CORRECTA".green());

            std::fs::copy(target_backup, &asar_path)?;
            println!("{}", "🎉 ¡Restauración completada con éxito! Antigravity está en su estado original.".bold().green());
        }

        Commands::Doctor => {
            println!("{}", "🩺 Diagnóstico de Entorno Antigravity Glassmorphism:".bold().cyan());
            println!("--------------------------------------------------");

            let asar_path = find_antigravity_asar(None);
            match asar_path {
                Some(p) => println!("  • Antigravity app.asar:     {} ({})", "✅ Detectado".green(), p.display()),
                None => println!("  • Antigravity app.asar:     {}", "❌ No encontrado".red()),
            }

            let css_path = get_custom_css_path();
            if css_path.exists() {
                let size = std::fs::metadata(&css_path).map(|m| m.len()).unwrap_or(0);
                println!("  • Estilo wallpaper.css:     {} ({} bytes)", "✅ Activo".green(), size);
            } else {
                println!("  • Estilo wallpaper.css:     {}", "ℹ️ No configurado".yellow());
            }

            let devtools_port = get_devtools_port_file();
            if devtools_port.exists() {
                println!("  • Puerto DevTools CDP:      {} ({})", "✅ Activo".green(), devtools_port.display());
            } else {
                println!("  • Puerto DevTools CDP:      {}", "ℹ️ Cerrado (Requiere app abierta)".yellow());
            }

            let wall_dirs = get_wallpaper_dirs();
            println!("  • Carpetas de Wallpapers:   {} detectadas", wall_dirs.len());
            for d in wall_dirs {
                println!("    - {}", d.display());
            }
            println!("--------------------------------------------------");
        }
    }

    Ok(())
}
