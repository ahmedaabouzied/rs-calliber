use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp::sample::FromSample;
use hound::{WavSpec, WavWriter};
use rodio::Sample;
use rodio::Source;
use rodio::{OutputStream, Sink};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{self, AsyncWriteExt};

use std::sync::mpsc::Sender;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use super::freq;

pub fn get_input_devices() -> Result<cpal::InputDevices<cpal::Devices>, cpal::DevicesError> {
    let host = cpal::default_host();
    host.input_devices()
}

pub fn get_output_devices() -> Result<cpal::OutputDevices<cpal::Devices>, cpal::DevicesError> {
    let host = cpal::default_host();
    host.output_devices()
}

fn select_input_device(device_name: String) -> cpal::Device {
    match device_name.as_str() {
        "Default" => {
            let host = cpal::default_host();
            host.default_input_device().unwrap()
        }
        _ => self::get_input_devices()
            .unwrap()
            .find(|d| d.name().unwrap() == device_name)
            .unwrap(),
    }
}

fn select_output_device(device_name: String) -> cpal::Device {
    match device_name.as_str() {
        "Default" => {
            let host = cpal::default_host();
            host.default_output_device().unwrap()
        }
        _ => self::get_output_devices()
            .unwrap()
            .find(|d| d.name().unwrap() == device_name)
            .unwrap(),
    }
}

pub fn capture_input(
    input_device_name: String,
    sample_rate: f32,
    buffer: Arc<Mutex<Vec<f32>>>,
    for_tx: Sender<f32>,
    is_playing: Arc<AtomicBool>,
) {
    if !is_playing.load(Ordering::SeqCst) {
        return;
    }
    let input_device = select_input_device(input_device_name);
    let config_range = input_device.default_input_config().unwrap();

    let data_clone = Arc::clone(&buffer);

    let input_stream = input_device
        .build_input_stream(
            &config_range.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut locked_data = data_clone.lock().unwrap();
                locked_data.extend_from_slice(data);
            },
            move |err| {
                eprintln!("An error occurred on the input stream: {}", err);
            },
            Option::None,
        )
        .unwrap();
    while is_playing.load(Ordering::SeqCst) {
        input_stream.play().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    input_stream.pause().unwrap();
    let locked_data = buffer.lock().unwrap();
    let ffr = freq::freq_of_resonance(locked_data.clone(), sample_rate);
    for_tx.send(ffr).unwrap();
}

pub fn play_output<S>(output_device_name: String, sound: S, stop_signal: Arc<AtomicBool>)
where
    S: Source + Send + 'static,
    f32: FromSample<S::Item>,
    S::Item: Sample + Send,
{
    let (_stream, stream_handle) =
        OutputStream::try_from_device(&select_output_device(output_device_name)).unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(sound);
    while stop_signal.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    sink.stop();
}

pub fn save_mono_vec_to_wav(
    data: &Vec<f32>,
    sample_rate: u32,
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = WavSpec {
        channels: 1, // Mono audio has 1 channel
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create(file_path, spec)?;

    for sample in data {
        // Convert f32 to i16
        let mono_sample = *sample * f32::MAX;

        writer.write_sample(mono_sample)?;
    }

    writer.finalize()?;
    Ok(())
}

pub async fn save_mono_vec_with_db_to_csv(
    data: &Vec<f32>,
    sample_rate: u32,
    file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(file_path).await?;
    file.write(b"Time (s),Sample Value,Amplitude (dB) \n")
        .await?; // Add header

    for (i, sample) in data.iter().enumerate() {
        let time = i as f32 / sample_rate as f32;
        let db_value = 20.0 * sample.abs().log10(); // Calculate amplitude in dB
        file.write(format!("{},{},{} \n", time, sample, db_value).as_bytes())
            .await?;
    }

    Ok(())
}
