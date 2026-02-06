//! 音频录制模块
//! 
//! 使用 cpal crate 录制麦克风音频

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 测试录音功能
pub fn test_recording() {
    println!("🎤 录音测试");
    println!("============");
    println!("");
    println!("⚠️  首次运行需要授予麦克风权限");
    println!("");
    
    // 获取默认音频主机
    let host = cpal::default_host();
    
    // 获取默认输入设备
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            println!("❌ 未找到输入设备");
            return;
        }
    };
    
    println!("📱 使用设备: {}", device.name().unwrap_or_default());
    
    // 获取默认输入配置
    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 获取配置失败: {:?}", e);
            return;
        }
    };
    
    println!("⚙️  采样率: {} Hz", config.sample_rate().0);
    println!("⚙️  通道数: {}", config.channels());
    println!("");
    println!("🔴 开始录音 5 秒...");
    
    match record_to_file(5) {
        Ok(path) => {
            println!("");
            println!("✅ 录音完成！");
            println!("📁 文件保存至: {}", path);
        }
        Err(e) => {
            println!("❌ 录音失败: {}", e);
        }
    }
}

/// 录音到文件
/// 
/// # Arguments
/// * `duration_secs` - 录音时长（秒）
/// 
/// # Returns
/// 录音文件路径
pub fn record_to_file(duration_secs: u64) -> Result<String, String> {
    let host = cpal::default_host();
    
    let device = host.default_input_device()
        .ok_or("未找到输入设备")?;
    
    let config = device.default_input_config()
        .map_err(|e| format!("获取配置失败: {:?}", e))?;
    
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    
    // WAV 文件配置
    let spec = WavSpec {
        channels: channels,
        sample_rate: sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let output_path = format!("/tmp/aitotype_recording_{}.wav", 
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    
    let writer = WavWriter::create(&output_path, spec)
        .map_err(|e| format!("创建文件失败: {:?}", e))?;
    
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_clone = writer.clone();
    
    let err_fn = |err| eprintln!("录音错误: {:?}", err);
    
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut guard) = writer_clone.lock() {
                        if let Some(ref mut w) = *guard {
                            for &sample in data {
                                let sample_i16 = (sample * i16::MAX as f32) as i16;
                                let _ = w.write_sample(sample_i16);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut guard) = writer_clone.lock() {
                        if let Some(ref mut w) = *guard {
                            for &sample in data {
                                let _ = w.write_sample(sample);
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
    
    std::thread::sleep(Duration::from_secs(duration_secs));
    
    drop(stream);
    
    // 完成写入
    if let Ok(mut guard) = writer.lock() {
        if let Some(w) = guard.take() {
            w.finalize().map_err(|e| format!("保存失败: {:?}", e))?;
        }
    }
    
    Ok(output_path)
}
