use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp::sample::FromSample;
use rodio::Sample;
use rodio::Source;
use rodio::{OutputStream, Sink};

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

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

pub fn capture_input(input_device_name: String, sample_rate: f32, duration: f32, tx: Sender<f32>) {
    let input_device = select_input_device(input_device_name);
    let config_range = input_device.default_input_config().unwrap();

    let data = Arc::new(Mutex::new(Vec::new())); // Will live along this function.
    let data_clone = Arc::clone(&data); // To be moved to a lambda.

    let input_stream = input_device
        .build_input_stream(
            &config_range.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut locked_data = data_clone.lock().unwrap();
                locked_data.extend_from_slice(data);
            }, // data_clone is dropped here.
            move |err| {
                eprintln!("An error occurred on the input stream: {}", err);
            },
            Option::None,
        )
        .unwrap();
    input_stream.play().unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f32(duration));
    let locked_data = data.lock().unwrap();
    let ffr = freq::freq_of_resonance(locked_data.clone(), sample_rate);
    tx.send(ffr).unwrap();
}

pub fn play_output<S>(output_device_name: String, sound: S)
where
    S: Source + Send + 'static,
    f32: FromSample<S::Item>,
    S::Item: Sample + Send,
{
    let (_stream, stream_handle) =
        OutputStream::try_from_device(&select_output_device(output_device_name)).unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(sound);
    sink.sleep_until_end();
}
