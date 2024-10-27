//! AAC decoder for MPEG-4 (MP4, M4A etc) and AAC files. Supports rodio.
use fdk_aac::dec::{Decoder as AacDecoder, DecoderError, Transport};
use std::io::Read;
use std::time::Duration;
use std::{error, fmt, io};

/// Redlux error
#[derive(Debug)]
pub enum Error {
  /// Error reading header of file
  FileHeaderError,
  /// Unable to get information about a track, such as audio profile, sample
  /// frequency or channel config.
  TrackReadingError,
  /// Unsupported audio object type
  UnsupportedObjectType,
  /// Unable to find track in file
  TrackNotFound,
  /// Error decoding track
  TrackDecodingError(DecoderError),
  /// Error getting samples
  SamplesError,
  /// Error from the underlying reader R
  ReaderError(io::Error),
}

impl error::Error for Error {}

impl Error {
  pub fn message(&self) -> &'static str {
    match &self {
      Error::FileHeaderError => "Error reading file header",
      Error::TrackReadingError => "Error reading file track info",
      Error::UnsupportedObjectType => "Unsupported audio object type",
      Error::TrackNotFound => "Unable to find track in file",
      Error::TrackDecodingError(_) => "Error decoding track",
      Error::SamplesError => "Error reading samples",
      Error::ReaderError(_) => "Error reading file",
    }
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.message())
  }
}

/// File container format
pub enum Format {
  Aac,
}

/// Underlying reader
pub enum Reader<R> {
  AacReader(R),
}

pub struct Decoder<R>
where
    R: Read,
{
  reader: Reader<R>,
  aac_decoder: AacDecoder,
  bytes: Vec<u8>,
  current_pcm_index: usize,
  current_pcm: Vec<i16>,
  /// If there's an error while iterating over the Decoder, that error is added here
  pub iter_error: Option<Error>,
}

impl<R> Decoder<R>
where
    R: Read,
{
  /// Create from an aac buffer
  pub fn new_aac(reader: R) -> Self {
    let aac_decoder = AacDecoder::new(Transport::Adts);
    Decoder {
      reader: Reader::AacReader(reader),
      aac_decoder,
      bytes: Vec::new(),
      current_pcm_index: 0,
      current_pcm: Vec::new(),
      iter_error: None,
    }
  }

  #[inline]
  pub fn current_frame_len(&self) -> Option<usize> {
    let frame_size: usize = self.aac_decoder.decoded_frame_size();
    Some(frame_size)
  }
  #[inline]
  pub fn channels(&self) -> u16 {
    let num_channels: i32 = self.aac_decoder.stream_info().numChannels;
    num_channels as _
  }
  #[inline]
  pub fn sample_rate(&self) -> u32 {
    let sample_rate: i32 = self.aac_decoder.stream_info().sampleRate;
    sample_rate as _
  }
  #[inline]
  pub fn total_duration(&self) -> Option<Duration> {
    None
  }
  /// Consume and return the next sample, or None when finished
  #[inline]
  pub fn decode_next_sample(&mut self) -> Result<Option<i16>, Error> {
    if self.current_pcm_index == self.current_pcm.len() {
      if self.current_frame_len().unwrap() == 0 && !self.current_pcm_index == 0 {
        return Ok(None);
      }

      let mut pcm = vec![0; 8192];
      let result = match self.aac_decoder.decode_frame(&mut pcm) {
        Err(DecoderError::NOT_ENOUGH_BITS) | Err(DecoderError::TRANSPORT_SYNC_ERROR) => {
          match &mut self.reader {
            // aac
            Reader::AacReader(aac_reader) => {
              let old_bytes_len = self.bytes.len();
              let mut new_bytes = vec![0; 8192 - old_bytes_len];
              let bytes_read = match aac_reader.read(&mut new_bytes) {
                Ok(bytes_read) => bytes_read,
                Err(err) => return Err(Error::ReaderError(err)),
              };
              if bytes_read == 0 {
                return Err(Error::SamplesError);
              }
              // aac files already have adts headers
              self.bytes.extend(new_bytes);
            }
          }
          let bytes_filled = match self.aac_decoder.fill(&self.bytes) {
            Ok(bytes_filled) => bytes_filled,
            Err(err) => return Err(Error::TrackDecodingError(err)),
          };
          self.bytes = self.bytes[bytes_filled..].to_vec();
          self.aac_decoder.decode_frame(&mut pcm)
        }
        val => val,
      };
      if let Err(err) = result {
        return Err(Error::TrackDecodingError(err));
      }
      let decoded_frame_size = self.aac_decoder.decoded_frame_size();
      if decoded_frame_size < pcm.len() {
        let _ = pcm.split_off(decoded_frame_size);
      }
      self.current_pcm = pcm;
      self.current_pcm_index = 0;
    }
    let value = self.current_pcm[self.current_pcm_index];
    self.current_pcm_index += 1;

    Ok(Some(value))
  }
}

impl<R> Iterator for Decoder<R>
where
    R: Read,
{
  type Item = i16;
  /// Runs decode_next_sample and returns the sample from that. Once the
  /// iterator is finished, it returns None. If there's an error, it's added
  /// to the iter_error error.
  #[inline]
  fn next(&mut self) -> Option<i16> {
    match self.decode_next_sample() {
      Ok(sample) => sample,
      Err(err) => {
        self.iter_error = Some(err);
        Some(0)
      }
    }
  }
}

impl<R> rodio::Source for Decoder<R>
where
    R: Read,
{
  #[inline]
  fn current_frame_len(&self) -> Option<usize> {
    self.current_frame_len()
  }
  #[inline]
  fn channels(&self) -> u16 {
    self.channels()
  }
  #[inline]
  fn sample_rate(&self) -> u32 {
    self.sample_rate()
  }
  #[inline]
  fn total_duration(&self) -> Option<Duration> {
    self.total_duration()
  }
}
