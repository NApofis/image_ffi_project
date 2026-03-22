use std::path::PathBuf;
use clap::Parser;
use anyhow::Result;
use image_processor::plugin_loader::{write_log, PluginLoader};
use image_processor::cli::{is_valid_json, prepare_image_png, check_file, check_dir, prepare_lib_path, LIB_EXTENSION, save_image};
use image_processor::error::ImageError;

#[derive(Debug, Parser)]
#[command(
    name = "Image Processor",
    version,
    about = "Cli для работы обработки изображений в формате PNG"
)]
struct Cli {
    /// Путь до исходного изображения
    #[arg(long)]
    input: PathBuf,

    /// Путь для сохранения обработанного изображения
    #[arg(long)]
    output: PathBuf,

    /// Название плагина без расширения
    #[arg(long)]
    plugin: String,

    /// Путь до файла с параметрами плагина
    #[arg(long)]
    params: PathBuf,

    /// Путь до плагина. По умолчанию ../target/debug
    #[arg(long)]
    plugin_path: Option<PathBuf>,
}


fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();
    log::info!("Старт приложения");

    let cli = Cli::parse();

    log::info!("Проверка параметров");
    check_file(&cli.input, "png")?;
    let filename = cli.input.file_name().ok_or_else(||{
        ImageError::InvalidParameter("не удалось определить название картинки".to_string())
    })?.to_str().ok_or_else(||{
        ImageError::InvalidParameter("не удалось прочитать название картинки".to_string())
    })?;
    let mut new_filename = "new_".to_owned();
    new_filename.push_str(filename);

    check_dir(&cli.output)?;
    check_file(&cli.params, "json")?;
    let param_string = is_valid_json(&cli.params)?;
    let plugin = prepare_lib_path(cli.plugin_path, cli.plugin.clone())?;
    check_file(&plugin, LIB_EXTENSION)?;

    let mut image_data = prepare_image_png(&cli.input)?;

    let plugin = PluginLoader::new(&plugin)?;
    log::info!("Библиотека плагина загружена");

    let plugin_interface = plugin.interface()?;
    log::info!("Функция process_image найдена");

    let c_params = std::ffi::CString::new(param_string)?;
    log::info!("Вызов плагина {}", &cli.plugin);

    unsafe {
        plugin_interface(image_data.width, image_data.height, image_data.data.as_mut_ptr(), c_params.as_ptr(), write_log);
    }
    save_image(&cli.output.join(new_filename), &image_data)?;
    log::info!("Приложения закончило работу");

    Ok(())
}
