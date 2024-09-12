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

pub fn capture_input(input_device_name: String, sample_rate: f32, duration: f32, tx: Sender<f32>) {
    let input_device = self::get_input_devices()
        .unwrap()
        .find(|d| d.name().unwrap() == input_device_name)
        .unwrap();
    let config_range = input_device.default_input_config().unwrap();
    let data = Arc::new(Mutex::new(Vec::new()));

    let data_clone = Arc::clone(&data);

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
    let (_stream, stream_handle) = OutputStream::try_from_device(
        &self::get_output_devices()
            .unwrap()
            .find(|d| d.name().unwrap() == output_device_name)
            .unwrap(),
    )
    .unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    // Play the sound directly on the device
    sink.append(sound);
    sink.sleep_until_end();
}
