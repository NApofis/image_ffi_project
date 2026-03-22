use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;
use image::{ImageReader, ImageBuffer, Rgba};

use crate::error::ImageError;
use crate::get_json;

const LIB_DEFAULT_PATH: &str = "../target/debug"; // Дефолтное расположение плагинов.

#[cfg(target_os = "linux")]
pub const LIB_EXTENSION: &str = "so"; // Расширение динамических библиотек для linux

#[cfg(target_os = "windows")]
pub const LIB_EXTENSION: &str = "dll"; // Расширение динамических библиотек для windows

/// 
/// Метод собирет название динамической библиотеки ориентированное на ОС 
/// 
/// # Arguments 
/// 
/// * `name`: имя библиотеки
/// 
/// returns: String - название динамической библиотеки 
/// 
pub fn lib_filename(name: &str) -> String {
    #[cfg(target_os = "linux")]
    {
        format!("lib{name}.{LIB_EXTENSION}")
    }

    #[cfg(target_os = "windows")]
    {
        format!("{name}.{LIB_EXTENSION}")
    }
}

/// Структура для хранения данных картинки
pub struct ImageData {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}



/// 
/// Проверит существования файла, его доступность и его расширение
/// 
/// # Arguments 
/// 
/// * `path`: пусть до файла
/// * `extension`: нужно расширение
/// 
/// returns: Result<(), ImageError> 
/// 
pub fn check_file(path: &PathBuf, extension: &str) -> Result<(), ImageError> {
    if !path.exists() {
        return Err(ImageError::InvalidParameter(format!("файл {} не существует", path.display())));
    }
    if !path.is_file() {
        return Err(ImageError::UnsupportedParameter(format!("путь {} не является файлом", path.display())));
    }

    let Some(real_extension) = path.extension() else {
        return Err(ImageError::UnsupportedParameter(format!("файл не имеет расширения {}", path.display())));
    };
    if real_extension != extension {
        return Err(ImageError::UnsupportedParameter(format!("файл имеет расширение {} а ожидалось {extension}", real_extension.display())));
    }
    Ok(())
}


/// 
/// Проверит существование директории
/// 
/// # Arguments 
/// 
/// * `dir`: путь до директории
/// 
/// returns: Result<(), ImageError> 
/// 
pub fn check_dir(dir: &PathBuf) -> Result<(), ImageError> {
    if !dir.exists() {
        return Err(ImageError::InvalidParameter(format!("путь {} не существует", dir.display())));
    }
    if !dir.is_dir() {
        return Err(ImageError::UnsupportedParameter(format!("путь {} не является папкой", dir.display())));
    }
    Ok(())
}

/// 
/// Проверить JSON на валидность
/// 
/// # Arguments 
/// 
/// * `path`: путь до json файла
/// 
/// returns: Result<String, ImageError> 
/// 
pub fn is_valid_json(path: &PathBuf) -> Result<String, ImageError> {
    let content = fs::read_to_string(path).map_err(|_| {
        ImageError::InvalidParameter(format!("ошибка чтения файла {}", path.display()))
    })?;

    let json = get_json(&content).ok_or_else(|| {
        ImageError::InvalidFileFormat(format!("содержимое файл {} записано не в json формате", path.display()))
    })?;

    Ok(json.to_string())
}

/// 
/// Прочитает данные изображения в структуру ImageData
/// 
/// # Arguments 
/// 
/// * `path`: путь до изображения
/// 
/// returns: Result<ImageData, ImageError> 
///
pub fn prepare_image_png(path: &PathBuf) -> Result<ImageData, ImageError> {
    let image = ImageReader::open(path).map_err(|_| {
        ImageError::InvalidFileFormat(format!("изображение {} повреждено", path.display()))
    })?.decode().map_err(|_| {
        ImageError::InvalidFileFormat(format!("не удалось разбить изображение {} на пиксели", path.display()))
    })?;

    let rgba = image.to_rgba8();

    let (width, height) = rgba.dimensions();
    let data = rgba.into_raw(); // Vec<u8>

    Ok(ImageData {
        width,
        height,
        data,
    })
}

/// 
/// Сохранит изображение 
/// 
/// # Arguments 
/// 
/// * `path`: путь до изображения вместе с новым названием файла
/// * `image`: данные изображения
/// 
/// returns: Result<(), ImageError> 
/// 
pub fn save_image(path: &PathBuf, image: &ImageData) -> Result<(), ImageError> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_vec(image.width, image.height, image.data.to_vec())
            .ok_or_else(|| {
                ImageError::SaveImageError("не удалось собрать картинку из сырых данных".to_string())
            })?;

    img.save(path).map_err(|_| {
        ImageError::SaveImageError("не удалось сохранить картинку".to_string())
    })
}

/// 
/// Собрать путь до динамической либы из частей
/// 
/// # Arguments 
/// 
/// * `path`: путь до либы
/// * `name`: название либы без расширения и префикса
/// 
/// returns: Result<PathBuf, ImageError> 
/// 
pub fn prepare_lib_path(path: Option<PathBuf>, name: String) -> Result<PathBuf, ImageError> {
    if name.is_empty() {
        return Err(ImageError::InvalidParameter("название плагина пустое".to_string()))
    }
;
    if PathBuf::from(name.as_str()).extension().is_some() {
        return Err(ImageError::UnsupportedParameter(format!("название плагина {name} передано с расширением")))
    }
    let plugin_name = lib_filename(name.as_str());

    Ok(if let Some(path) = path {
        path.join(plugin_name)
    } else {
        Path::new(LIB_DEFAULT_PATH).join(plugin_name)
    })
}