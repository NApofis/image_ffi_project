use std::fmt;
use std::fmt::Formatter;
use anyhow::Error;

/// Ошибки, которые могут возникнуть при работе приложения. Они будут выведены в консоль
pub enum ImageError {
    UnsupportedParameter(String),
    InvalidFileFormat(String),
    SaveImageError(String),
    InvalidParameter(String),
    LoadPluginError(String),
}
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::UnsupportedParameter(s) => write!(f, "Неверный формат консольных параметров: {}", s),
            ImageError::InvalidFileFormat(s) => write!(f, "Неподходящее содержимое файлов: {}", s),
            ImageError::InvalidParameter(s) => write!(f, "Ошибка разбора консольных параметров: {}", s),
            ImageError::LoadPluginError(s) => write!(f, "Ошибка загрузки плагина: {}", s),
            ImageError::SaveImageError(s) => write!(f, "Ошибка сохранения картинки: {}", s),
        }
    }
}

impl From<ImageError> for Error {
    fn from(value: ImageError) -> Self {
        Error::msg(value.to_string())
    }
}

impl From<libloading::Error> for ImageError {
    fn from(value: libloading::Error) -> Self {
        ImageError::LoadPluginError(value.to_string())
    }
}
