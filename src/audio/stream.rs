use std::io;
use std::io::Read;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct StreamPipe {
    pub buffer: Arc<std::sync::Mutex<Vec<u8>>>,
    position: usize,
}

impl StreamPipe {
    pub fn add(&mut self, data: &[u8]) {
        self.buffer.lock().unwrap().extend(data);
    }
    pub fn clear(&mut self) {
        self.buffer.lock().unwrap().clear();
    }
}
impl Read for StreamPipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut vec = self
            .buffer
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to lock mutex"))?;

        let remaining = vec.len() - self.position;
        let to_read = remaining.min(buf.len());

        // Copy data into buffer
        buf[..to_read].copy_from_slice(&vec[self.position..self.position + to_read]);

        // drain
        vec.drain(..to_read);
        Ok(to_read)
    }
}
