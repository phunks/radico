use log::info;
use crate::api::worker::rand;
use crate::audio::assets::ASSETS;
use crate::audio::sink::MusicStruct;
use crate::audio::stream::StreamPipe;
use rodio::Sink;

pub struct Player {
    sink: Sink,
    pipe: StreamPipe,
}

impl Default for Player {
    fn default() -> Self {
        let stream_handle = MusicStruct::new();
        let stream = StreamPipe::default();
        let mut pipe = stream.clone();
        let dec = redlux::Decoder::new_aac(stream);
        let sink = Sink::try_new(&stream_handle.stream_handle.unwrap()).unwrap();

        pipe.add(&ASSETS.get(rand()));

        sink.append(dec);
        Player { sink, pipe }
    }
}

impl Player {
    pub fn add(&mut self, buf: &[u8]) {
        self.pipe.add(buf);
    }

    pub fn volume(&mut self, level: char) {
        self.sink
            .set_volume((level.to_digit(10).unwrap() as f32 / 9_f32).powf(2.0));
    }

    pub fn buffer_length(&self) -> usize {
        self.pipe.buffer.lock().unwrap().len()
    }

    pub fn buffer_clear(&mut self) {
        info!("buffer clear\r");
        self.pipe.clear();
    }
}
