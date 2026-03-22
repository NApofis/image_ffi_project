use std::ffi::{CStr, CString};
use std::fmt;
use std::fmt::Formatter;
use std::os::raw::c_char;
pub use serde_json::Value as SerdeJsonValue;

#[cfg(feature = "cli")] // Отдельная фича что бы не тащить в плагины полный список зависимостей
pub mod error;

#[cfg(feature = "cli")]
pub mod plugin_loader;

#[cfg(feature = "cli")]
pub mod cli;

pub const CHANNELS: u8 = 4; // количество байт кодирующее один пиксель

/// Ошибки для плагинов что бы не приходилось создавать в каждом плагине свои
pub enum ImagePluginError {
    PluginError(String),
    ParameterError(String),
}

impl fmt::Display for ImagePluginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ImagePluginError::PluginError(s) => write!(f, "Ошибка выполнения8 плагина({})", s),
            ImagePluginError::ParameterError(s) => write!(f, "Ошибка разбора параметров({})", s),
        }
    }
}


///
/// Пробует распарсить строку в json параметры
///
/// # Arguments
///
/// * `json_str`: строка в которой лежит json данные
///
/// returns: Option<Value> json если получилось его десериализовать
///
pub fn get_json(json_str: &str) -> Option<serde_json::Value> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) else {
        return None
    };
    Some(json)
}


///
/// Метод для проверки параметров, что бы проще было в плагинах
///
/// # Arguments
///
/// * `rgba_data`: массив с данными пикселей
/// * `params`: параметры модуля которые должны быть в формате json
///
/// returns: Result<(), ImagePluginError>
///
pub fn check_unsafe_params(rgba_data: *mut u8, params: *const c_char) -> Result<(), ImagePluginError> {
    if rgba_data.is_null() {
        return Err(ImagePluginError::ParameterError("не переданы пиксели картинки".to_string()));
    }

    if params.is_null() {
        return Err(ImagePluginError::ParameterError("список параметров пустой".to_string()));
    }
    Ok(())
}

///
/// Проверить и получить размер картинки в байтах
///
/// # Arguments
///
/// * `width`: Ширина в пикселях
/// * `height`: Высота в пикселях
///
/// returns: Result<usize, ImagePluginError>
///
pub fn get_rgba_data_size(width: u32, height: u32) -> Result<usize, ImagePluginError> {

    let pixel_count = match (width as usize).checked_mul(height as usize) {
        Some(v) => Ok(v),
        None => Err(ImagePluginError::ParameterError("размер картинки слишком большой".to_string()))
    }?;

    Ok(match pixel_count.checked_mul(4) {
        Some(v) => Ok(v),
        None => Err(ImagePluginError::ParameterError("размер картинки слишком большой".to_string()))
    }?)
}

///
/// Распарсит параметры в формате json из си строки
///
/// # Arguments
///
/// * `params`: си строка
///
/// returns: Result<Value, ImagePluginError>
///
pub fn get_params_json(params: *const c_char) -> Result<serde_json::Value, ImagePluginError> {
    // SAFETY
    // - указатель params не должен быть nullpth
    // - указатель params должен указывать на char данные
    // - данные по указателю должны заканчиваться \0 символом
    let params_str = unsafe {
        CStr::from_ptr(params).to_str().map_err(|_| {
            ImagePluginError::PluginError("ошибка разбора строки параметров".to_string())
        })?
    };
    Ok(get_json(params_str).ok_or(ImagePluginError::PluginError("ошибка преобразования строки параметров в json".to_string())))?
}

/// Сигнатура функции для записи логов. Что бы в плагинах можно было писать в логи cli приложения 
pub type LogFn = unsafe extern "C" fn(level: u8, message: *const c_char);

///
/// Метод для записи логово в логер. Нужен для упрощения записи, что бы не создавать в коде
///
/// # Arguments
///
/// * `logger`: Функция логирования
/// * `level`: Уровень логирования для функции
/// * `text`: Текст логов
///
pub fn send_log(logger: LogFn, level: u8, text: &str) {
    if let Ok(c_text) = CString::new(text) {
        // SAFETY
        // - logger долже существовать
        // - сигнатура logger должна соответсвовать вызову
        unsafe { logger(level, c_text.as_ptr()) };
    }
}