use std::os::raw::c_char;
use std::path::Path;
use libloading::Library;
use crate::error::*;
use crate::LogFn;

const METHOD_NAME: &str = "process_image"; // Дефолтное название метода расширения

/// Сигнатура метода которая должна быть реализована в плагине
pub type ProcessImageFn = unsafe extern "C" fn(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
    log_fn: LogFn,
);

/// Структура для загрузки либы и подготовки символа для вызова
pub struct PluginLoader {
    plugin: Library,
}

impl PluginLoader {
    pub fn new(filename: &Path) -> Result<Self, libloading::Error> {
        // SAFETY:
        // - загрузка динамической библиотеки может выполнять произвольный код (например, static init)
        // - мы доверяем этой библиотеке (она не должна содержать UB или вредоносный код)
        Ok(Self {
            plugin: unsafe { Library::new(filename) }?,
        })
    }
    pub fn interface(&self) -> Result<ProcessImageFn, ImageError> {
        // SAFETY:
        // - символ с именем METHOD_NAME должен существовать в библиотеке
        // - он должен иметь точную сигнатуру ProcessImageFn (включая ABI "C")
        // - несоответствие сигнатуры приведёт к UB при вызове
        let symbol = unsafe {
            self.plugin.get::<ProcessImageFn>(METHOD_NAME.as_bytes())
        }.map_err(|_| {
            ImageError::LoadPluginError(format!("В библиотеке не найдена функция {METHOD_NAME}"))
        })?;

        Ok(*symbol)
    }
}


///
/// Метод для записи в логи который будет передан плагину в качестве логера
///
/// # Arguments
///
/// * `level`: 2-warning, 3-error, остальное-info
/// * `msg`: сообщение
///
/// returns: ()
///
pub unsafe extern "C" fn write_log(level: u8, msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    // SAFETY
    // - указатель msg не должен быть nullpth
    // - указатель должен указывать на char данные
    // - данные по указателю должны заканчиваться \0 символом
    let msg = match unsafe { std::ffi::CStr::from_ptr(msg) }.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    match level {
        2 => log::warn!("{msg}"),
        3 => log::error!("{msg}"),
        _ => log::info!("{msg}"),
    }
}