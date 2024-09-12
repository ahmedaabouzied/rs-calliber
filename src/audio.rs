use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dasp::sample::FromSample;
use rodio::Sample;
use rodio::Source;
use rodio::{OutputStream, Sink};

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use super::freq;

pub fn capture_input(sample_rate: f32, duration: f32, tx: Sender<f32>) {
    let host = cpal::default_host();
    let input_device = host.default_input_device().unwrap();
    println!("Input device: {}", input_device.name().unwrap());
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

pub fn play_output<S>(sound: S)
where
    S: Source + Send + 'static,
    f32: FromSample<S::Item>,
    S::Item: Sample + Send,
{
    // Play the sound for 2 seconds through the speakers
    // Get an output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    println!("Output device: Default");
    let sink = Sink::try_new(&stream_handle).unwrap();
    // Play the sound directly on the device
    sink.append(sound);
    sink.sleep_until_end();
}
