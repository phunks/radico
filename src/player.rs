use crate::sink::MusicStruct;
use crate::stream::StreamPipe;
use rodio::Sink;

pub struct Player {
    sink: Sink,
    pipe: StreamPipe,
}

impl Player {
    pub(crate) fn new() -> Player {
        let stream_handle = MusicStruct::new();
        let stream = StreamPipe::default();
        let pipe = stream.to_owned();
        let dec = redlux::Decoder::new_aac(stream);
        let sink = Sink::try_new(&stream_handle.stream_handle.unwrap()).unwrap();

        sink.append(dec);
        Player { sink, pipe }
    }

    pub fn add(&mut self, buf: &[u8]) {
        self.pipe.add(buf);
    }

    pub fn volume(&mut self, vol: u8) {
        self.sink.set_volume((vol as f32 / 9_f32).powf(2.0));
    }

    #[allow(dead_code)]
    pub fn buffer_length(&self) -> usize {
        self.pipe.buffer.lock().unwrap().len()
    }

    pub fn buffer_clear(&mut self) {
        self.pipe.clear();
    }
}
