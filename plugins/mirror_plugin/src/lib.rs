use std::os::raw::c_char;
use std::slice;
use image_processor::{check_unsafe_params, get_params_json, get_rgba_data_size, ImagePluginError, SerdeJsonValue, CHANNELS, send_log, LogFn};

///
/// Экспортируемый метод плагина с которого начинается выполнение
///
/// # Arguments 
///
/// * `width`: ширина в пикселях
/// * `height`: высота в пикселях
/// * `rgba_data`: массив байт картинки
/// * `params`: параметры для плагина
/// * `log`: логгер для сообщений об статусе работы плагина
///
/// returns: () - метод ничего не возвращает, что бы не делать завязку приложения на работу плагина. Ошибки пишутся в логи. Но плагин все равно может упасть c panic
///
#[unsafe(no_mangle)]
pub extern "C" fn process_image(width: u32, height: u32, rgba_data: *mut u8, params: *const c_char, log: LogFn) {

    send_log(log, 1,"Модуль начал работу");

    match check_unsafe_params(rgba_data, params) {
        Ok(_) => (),
        Err(err) => {
            send_log(log, 1,err.to_string().as_str());
            return;
        }
    }

    let byte_len = match get_rgba_data_size(width, height)
    {
        Ok(v) => v,
        Err(err) => {
            send_log(log, 1,err.to_string().as_str());
            return;
        }
    };

    let rgba: &mut [u8] = unsafe {
        slice::from_raw_parts_mut(rgba_data, byte_len)
    };
    let params = unsafe {
        match get_params_json(params)
        {
            Ok(v) => v,
            Err(err) => {
                send_log(log, 1, err.to_string().as_str());
                return;
            }
        }
    };

    send_log(log, 1,"Проверка параметров прошла успешно");
    match process_image_safe(width, height, rgba, params) {
        Ok(_) => (),
        Err(err) => {
            send_log(log, 3,err.to_string().as_str());
            return;
        }
    }
    send_log(log, 1,"Модуль закончил работу");

}

/// Безопасный метод в котором начинается выполнение работы
fn process_image_safe(
    width: u32,
    height: u32,
    rgba: &mut [u8],
    params: SerdeJsonValue
) -> Result<(), ImagePluginError> {
    let (do_horizontal, do_vertical) = parse_mirror_params(params)?;

    if do_horizontal.is_some() {
        mirror_vertical(width as usize, height as usize, rgba);
    }

    if do_vertical.is_some() {
        mirror_horizontal(width as usize, height as usize, rgba);
    }

    Ok(())
}

/// Проверит параметры в конфиге
fn parse_mirror_params(params: SerdeJsonValue) -> Result<(Option<bool>, Option<bool>), ImagePluginError> {
    let get_param = |name: &str| -> Result<Option<bool>, ImagePluginError> {
        match params.get(name) {
            Some(r) => {
                match r.as_bool() {
                    Some(v) => Ok(Some(v)),
                    None => Err(ImagePluginError::ParameterError(format!("значение параметра {name} должно быть булевым")))
                }
            },
            None => Ok(None)
        }
    };
    let horizontal  = get_param("horizontal")?;
    let vertical = get_param("vertical")?;
    if horizontal.is_none() && vertical.is_none() {
        return Err(ImagePluginError::PluginError("должно быть выбрано горизонтальное или вертикальное отображение".to_string()));
    }
    Ok((horizontal, vertical))
}

/// Метод для отзеркаливания по горизонтали
fn mirror_horizontal(width: usize, height: usize, rgba: &mut [u8]) {

    for y in 0..height {
        for x in 0..(width / 2) {
            let left = (y * width + x) * CHANNELS as usize;
            let right = (y * width + (width - 1 - x)) * CHANNELS as usize;

            for c in 0..(CHANNELS as usize) {
                rgba.swap(left + c, right + c);
            }
        }
    }
}

/// Метод для отзеркаливания по вертикали
fn mirror_vertical(width: usize, height: usize, rgba: &mut [u8]) {
    let row_len = width * CHANNELS as usize;

    for y in 0..(height / 2) {
        let top = y * row_len;
        let bottom = (height - 1 - y) * row_len;

        for i in 0..row_len {
            rgba.swap(top + i, bottom + i);
        }
    }
}