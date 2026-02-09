//! 音频录制模块
//!
//! 提供麦克风录音功能，使用 cpal crate 捕获音频数据
//! 
//! 参考 poc 实现，确保采样率和通道数与设备配置一致

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// 全局录音状态
lazy_static::lazy_static! {
    static ref IS_RECORDING: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    static ref CURRENT_PATH: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    static ref RECORDING_DONE: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

/// 获取录音状态
pub fn is_recording() -> bool {
    IS_RECORDING.load(Ordering::SeqCst)
}

fn build_output_path() -> Result<String, String> {
    let mut temp_dir = std::env::temp_dir();
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("创建临时目录失败: {:?}", e))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("系统时间异常: {:?}", e))?
        .as_millis();

    temp_dir.push(format!(
        "aitotype_recording_{}_{}.wav",
        std::process::id(),
        timestamp
    ));

    Ok(temp_dir.to_string_lossy().to_string())
}

/// 开始录音
pub fn start_recording() -> Result<(), String> {
    if IS_RECORDING.load(Ordering::SeqCst) {
        return Err("已经在录音中".to_string());
    }

    // 等待上次录音完成
    let mut wait_count = 0;
    while !RECORDING_DONE.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(50));
        wait_count += 1;
        if wait_count > 40 { // 最多等待 2 秒
            return Err("上次录音尚未完成".to_string());
        }
    }

    // 使用系统临时目录，兼容 macOS / Linux / Windows。
    let output_path = build_output_path()?;

    // 保存路径
    {
        let mut path = CURRENT_PATH.lock().unwrap();
        *path = Some(output_path.clone());
    }

    IS_RECORDING.store(true, Ordering::SeqCst);
    RECORDING_DONE.store(false, Ordering::SeqCst);

    // 在新线程中进行录音
    thread::spawn(move || {
        let result = do_recording(&output_path);
        if let Err(e) = result {
            eprintln!("录音错误: {}", e);
        }
        RECORDING_DONE.store(true, Ordering::SeqCst);
    });

    Ok(())
}

/// 实际录音逻辑
fn do_recording(output_path: &str) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("未找到麦克风设备")?;

    let config = device.default_input_config()
        .map_err(|e| format!("获取配置失败: {:?}", e))?;

    // 使用设备的实际采样率和通道数
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    println!("📱 录音设备: {}", device.name().unwrap_or_default());
    println!("⚙️  采样率: {} Hz, 通道: {}", sample_rate, channels);

    // WAV 文件配置 - 使用设备的实际参数
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = WavWriter::create(output_path, spec)
        .map_err(|e| format!("创建文件失败: {:?}", e))?;
    
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_clone = writer.clone();

    let err_fn = |err| eprintln!("录音错误: {:?}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |samples: &[f32], _: &cpal::InputCallbackInfo| {
                    if IS_RECORDING.load(Ordering::SeqCst) {
                        if let Ok(mut guard) = writer_clone.lock() {
                            if let Some(ref mut w) = *guard {
                                for &sample in samples {
                                    let sample_i16 = (sample * i16::MAX as f32) as i16;
                                    let _ = w.write_sample(sample_i16);
                                }
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let writer_clone2 = writer.clone();
            device.build_input_stream(
                &config.into(),
                move |samples: &[i16], _: &cpal::InputCallbackInfo| {
                    if IS_RECORDING.load(Ordering::SeqCst) {
                        if let Ok(mut guard) = writer_clone2.lock() {
                            if let Some(ref mut w) = *guard {
                                for &sample in samples {
                                    let _ = w.write_sample(sample);
                                }
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
        }
        _ => return Err("不支持的采样格式".to_string()),
    }.map_err(|e| format!("创建流失败: {:?}", e))?;

    stream.play().map_err(|e| format!("启动录音失败: {:?}", e))?;

    // 持续录音直到停止
    while IS_RECORDING.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(50));
    }

    // 停止流
    drop(stream);

    // 完成写入
    if let Ok(mut guard) = writer.lock() {
        if let Some(w) = guard.take() {
            w.finalize().map_err(|e| format!("保存失败: {:?}", e))?;
        }
    }

    println!("✅ 录音完成: {}", output_path);
    Ok(())
}

/// 停止录音并返回音频数据路径
pub fn stop_recording() -> Result<String, String> {
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return Err("当前没有在录音".to_string());
    }

    // 停止录音
    IS_RECORDING.store(false, Ordering::SeqCst);
    
    // 等待录音线程完成
    let mut wait_count = 0;
    while !RECORDING_DONE.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(50));
        wait_count += 1;
        if wait_count > 40 { // 最多等待 2 秒
            return Err("等待录音完成超时".to_string());
        }
    }

    // 返回文件路径
    let path = CURRENT_PATH.lock().unwrap();
    path.clone().ok_or_else(|| "没有录音文件".to_string())
}

/// 获取当前音频电平 (0.0 - 1.0)
pub fn get_audio_level() -> f32 {
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return 0.0;
    }
    // 返回模拟值
    0.3 + (rand_simple() * 0.5)
}

/// 简单的伪随机数生成
fn rand_simple() -> f32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f32 / 1000.0
}
