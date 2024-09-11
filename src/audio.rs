use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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
