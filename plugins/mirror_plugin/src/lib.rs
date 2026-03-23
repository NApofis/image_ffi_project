use std::os::raw::c_char;
use std::slice;
use image_processor::{check_unsafe_params, get_params_json, get_rgba_data_size, ImagePluginError, CHANNELS, send_log, LogFn, SerdeJson};

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
pub unsafe extern "C" fn process_image(width: u32, height: u32, rgba_data: *mut u8, params: *const c_char, log: LogFn) {

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
    // SAFETY
    // - указатель rgba_data не должен быть nullpth
    // - длина rgba_data должа быть больше или равна byte_len
    // - указатель rgba_data должен содержать данные u8
    let rgba: &mut [u8] = unsafe {
        slice::from_raw_parts_mut(rgba_data, byte_len)
    };
    let params = match get_params_json(params)
    {
        Ok(v) => v,
        Err(err) => {
            send_log(log, 1, err.to_string().as_str());
            return;
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
    params: SerdeJson::Value
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
fn parse_mirror_params(params: SerdeJson::Value) -> Result<(Option<bool>, Option<bool>), ImagePluginError> {
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

/// Метод для отзеркаливания по вертикали
fn mirror_vertical(width: usize, height: usize, rgba: &mut [u8]) {

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

/// Метод для отзеркаливания по горизонтали
fn mirror_horizontal(width: usize, height: usize, rgba: &mut [u8]) {
    let row_len = width * CHANNELS as usize;

    for y in 0..(height / 2) {
        let top = y * row_len;
        let bottom = (height - 1 - y) * row_len;

        for i in 0..row_len {
            rgba.swap(top + i, bottom + i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba_pixels(pixels: &[[u8; 4]]) -> Vec<u8> {
        pixels.iter().flat_map(|p| p.iter().copied()).collect()
    }

    #[test]
    fn mirror_horizontal_works_for_normal_and_edge_cases() {
        let mut rgba = rgba_pixels(&[
            [1, 0, 0, 255],
            [2, 0, 0, 255],
            [3, 0, 0, 255],
            [4, 0, 0, 255],
        ]);

        mirror_horizontal(2, 2, &mut rgba);

        let expected = rgba_pixels(&[
            [3, 0, 0, 255],
            [4, 0, 0, 255],
            [1, 0, 0, 255],
            [2, 0, 0, 255],
        ]);

        assert_eq!(rgba, expected);

        let mut one_row = rgba_pixels(&[
            [10, 0, 0, 255],
            [20, 0, 0, 255],
            [30, 0, 0, 255],
        ]);

        let original = one_row.clone();
        mirror_horizontal(3, 1, &mut one_row);

        assert_eq!(one_row, original);
    }

    #[test]
    fn mirror_vertical_works_for_normal_and_edge_cases() {
        let mut rgba = rgba_pixels(&[
            [1, 0, 0, 255],
            [2, 0, 0, 255],
            [3, 0, 0, 255],
            [4, 0, 0, 255],
        ]);

        mirror_vertical(2, 2, &mut rgba);

        let expected = rgba_pixels(&[
            [2, 0, 0, 255],
            [1, 0, 0, 255],
            [4, 0, 0, 255],
            [3, 0, 0, 255],
        ]);

        assert_eq!(rgba, expected);

        let mut one_col = rgba_pixels(&[
            [10, 0, 0, 255],
            [20, 0, 0, 255],
            [30, 0, 0, 255],
        ]);

        let original = one_col.clone();
        mirror_vertical(1, 3, &mut one_col);

        assert_eq!(one_col, original);
    }

    #[test]
    fn parse_mirror_params_works_for_valid_and_invalid_cases() {
        let params = SerdeJson::json!({
            "horizontal": true,
            "vertical": false
        });
        let result = parse_mirror_params(params).unwrap();
        assert_eq!(result, (Some(true), Some(false)));

        let params = SerdeJson::json!({
            "horizontal": true
        });
        let result = parse_mirror_params(params).unwrap();
        assert_eq!(result, (Some(true), None));

        let params = SerdeJson::json!({
            "vertical": true
        });
        let result = parse_mirror_params(params).unwrap();
        assert_eq!(result, (None, Some(true)));

        let params = SerdeJson::json!({});
        let err = parse_mirror_params(params).unwrap_err();
        assert!(matches!(err, ImagePluginError::PluginError(_)));

        let params = SerdeJson::json!({
            "horizontal": "true"
        });
        let err = parse_mirror_params(params).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ошибка разбора параметров(значение параметра horizontal должно быть булевым)"
        );

        let params = SerdeJson::json!({
            "vertical": 1
        });
        let err = parse_mirror_params(params).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ошибка разбора параметров(значение параметра vertical должно быть булевым)"
        );
    }
}