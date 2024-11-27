use anyhow::Result;
use rodio::{OutputStream, OutputStreamHandle};
use std::marker::PhantomData;

#[allow(unused_imports)]
use cpal::traits::HostTrait;
#[allow(unused_imports)]
use rodio::DeviceTrait;

pub struct MusicStruct<'a> {
    pub stream_handle: Option<OutputStreamHandle>,
    phantom: PhantomData<&'a ()>,
}

impl MusicStruct<'_> {
    pub fn new() -> Self {
        let (stream, stream_handle) = get_output_stream().unwrap();

        std::mem::forget(stream);
        MusicStruct {
            stream_handle: Some(stream_handle),
            phantom: PhantomData,
        }
    }
}

fn get_output_stream() -> Result<(OutputStream, OutputStreamHandle)> {
    #[cfg(target_family = "windows")]
    {
        let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host");
        #[allow(clippy::useless_conversion)]
        if host.output_devices().unwrap().into_iter().count() > 0 {
            let devices = host.output_devices()?;
            let b = String::from("ASIO4ALL v2");
            let dev = devices.into_iter().find(|x| x.name().unwrap() == b).unwrap();
            Ok(OutputStream::try_from_device(&dev)?)
        } else {
            // WASAPI
            Ok(OutputStream::try_default()?)
        }
    }
    #[cfg(target_family = "unix")]
    {
        Ok(OutputStream::try_default()?)
    }
}

pub fn list_host_devices() {
    #[cfg(target_family = "unix")]
    let host = cpal::default_host();
    #[cfg(target_family = "windows")]
    let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host");
    let devices = host.output_devices().unwrap();
    for device in devices {
        let dev: rodio::Device = device;
        let dev_name: String = dev.name().unwrap();
        println!(" # Device : {}", dev_name);
    }
}
