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

    send_log(log, 1, "Модуль начал работу");

    match check_unsafe_params(rgba_data, params) {
        Ok(_) => (),
        Err(err) => {
            send_log(log, 3, err.to_string().as_str());
            return;
        }
    }

    let byte_len = match get_rgba_data_size(width, height)
    {
        Ok(v) => v,
        Err(err) => {
            send_log(log, 3, err.to_string().as_str());
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
            send_log(log, 3, err.to_string().as_str());
            return;
        }
    };

    send_log(log, 1, "Проверка параметров прошла успешно");
    match process_image_safe(width, height, rgba, params) {
        Ok(_) => (),
        Err(err) => {
            send_log(log, 3, err.to_string().as_str());
            return;
        }
    }
    send_log(log, 1, "Модуль закончил работу");
}


/// Безопасный метод в котором начинается выполнение работы
fn process_image_safe(
    width: u32,
    height: u32,
    rgba: &mut [u8],
    params: SerdeJson::Value,
) -> Result<(), ImagePluginError> {
    let (radius, iterations) = parse_blur_params(params)?;
    blur_weighted(width, height, rgba, radius, iterations);
    Ok(())
}

/// Проверит параметры в конфиге
fn parse_blur_params(params: SerdeJson::Value) -> Result<(u32, u32), ImagePluginError> {
    let get_param = |name: &str| -> Result<u32, ImagePluginError> {
        let result = match params.get(name) {
            Some(r) => Ok(r.as_u64().ok_or_else(|| ImagePluginError::PluginError(format!("значение параметра {name} должно быть положительным числом")))?),
            None => Err(ImagePluginError::ParameterError(format!("в параметрах отсутствует значение для {name}")))
        }?;
        if result == 0 {
            return Err(ImagePluginError::PluginError(format!("значение параметра {name} должно быть больше 0")))
        }
        Ok(result as u32)
    };

    Ok((get_param("radius")?, get_param("iterations")?))
}

/// Метод выполняющий основную работу плагина
fn blur_weighted(
    width: u32,
    height: u32,
    rgba: &mut [u8],
    radius: u32,
    iterations: u32,
) {
    let mut src = rgba.to_vec();
    let mut dst = vec![0u8; rgba.len()];

    let radius_i = radius as isize;
    let width_i = width as isize;
    let height_i = height as isize;

    for _ in 0..iterations {
        for y in 0..height {
            for x in 0..width {
                let x_i = x as isize;
                let y_i = y as isize;

                let mut sum_r = 0.0f64;
                let mut sum_g = 0.0f64;
                let mut sum_b = 0.0f64;
                let mut sum_a = 0.0f64;
                let mut total_weight = 0.0f64;

                for dy in -radius_i..=radius_i {
                    for dx in -radius_i..=radius_i {
                        let nx = x_i + dx;
                        let ny = y_i + dy;

                        if nx < 0 || ny < 0 || nx >= width_i || ny >= height_i {
                            continue;
                        }

                        let distance = ((dx * dx + dy * dy) as f64).sqrt();

                        if distance > radius as f64 {
                            continue;
                        }

                        let weight = 1.0 / (1.0 + distance);

                        let neighbor_index = (ny as usize * width as usize + nx as usize) * CHANNELS as usize;

                        sum_r += src[neighbor_index] as f64 * weight;
                        sum_g += src[neighbor_index + 1] as f64 * weight;
                        sum_b += src[neighbor_index + 2] as f64 * weight;
                        sum_a += src[neighbor_index + 3] as f64 * weight;
                        total_weight += weight;
                    }
                }

                let dst_index = (y * width + x) as usize * CHANNELS as usize;

                if total_weight > 0.0 {
                    dst[dst_index] = (sum_r / total_weight).round().clamp(0.0, 255.0) as u8;
                    dst[dst_index + 1] = (sum_g / total_weight).round().clamp(0.0, 255.0) as u8;
                    dst[dst_index + 2] = (sum_b / total_weight).round().clamp(0.0, 255.0) as u8;
                    dst[dst_index + 3] = (sum_a / total_weight).round().clamp(0.0, 255.0) as u8;
                } else {
                    dst[dst_index] = src[dst_index];
                    dst[dst_index + 1] = src[dst_index + 1];
                    dst[dst_index + 2] = src[dst_index + 2];
                    dst[dst_index + 3] = src[dst_index + 3];
                }
            }
        }

        std::mem::swap(&mut src, &mut dst);
    }

    rgba.copy_from_slice(&src);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rgba_pixels(pixels: &[[u8; 4]]) -> Vec<u8> {
        pixels.iter().flat_map(|p| p.iter().copied()).collect()
    }

    #[test]
    fn blur_weighted_works_for_basic_and_edge_cases() {
        let mut single = rgba_pixels(&[
            [10, 20, 30, 255],
        ]);
        let original = single.clone();

        blur_weighted(1, 1, &mut single, 3, 2);

        assert_eq!(single, original);

        // 3x1: проверяем, что blur реально меняет центральный/соседние пиксели,
        // но длина буфера сохраняется и alpha не ломается
        let mut row = rgba_pixels(&[
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            [0, 0, 0, 255],
        ]);

        blur_weighted(3, 1, &mut row, 1, 1);

        assert_eq!(row.len(), 3 * 4);

        // После blur картинка не должна остаться прежней
        assert_ne!(
            row,
            rgba_pixels(&[
                [0, 0, 0, 255],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
            ])
        );

        assert_eq!(row[3], 255);
        assert_eq!(row[7], 255);
        assert_eq!(row[11], 255);

        let left = row[0];
        let center = row[4];
        let right = row[8];

        assert!(center >= left);
        assert!(center >= right);
    }

    #[test]
    fn parse_blur_params_works_for_valid_and_invalid_cases() {
        let params = SerdeJson::json!({
            "radius": 3,
            "iterations": 2
        });

        let result = parse_blur_params(params).unwrap();
        assert_eq!(result, (3, 2));

        let params = SerdeJson::json!({
            "radius": 5
        });
        let err = parse_blur_params(params).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ошибка разбора параметров(в параметрах отсутствует значение для iterations)"
        );

        let params = SerdeJson::json!({
            "iterations": 4
        });
        let err = parse_blur_params(params).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ошибка разбора параметров(в параметрах отсутствует значение для radius)"
        );

        // пустой объект -> должны взяться значения по умолчанию
        let params = SerdeJson::json!({});
        let err = parse_blur_params(params).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ошибка разбора параметров(в параметрах отсутствует значение для radius)"
        );


        let params = SerdeJson::json!({
            "radius": "3",
            "iterations": 2
        });
        let err = parse_blur_params(params).unwrap_err();
        assert!(matches!(err, ImagePluginError::PluginError(_)));

        let params = SerdeJson::json!({
            "radius": 3,
            "iterations": "2"
        });
        let err = parse_blur_params(params).unwrap_err();
        assert!(matches!(err, ImagePluginError::PluginError(_)));
    }
}