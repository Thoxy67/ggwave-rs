#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

//! # ggwave-rs
//!
//! A Rust wrapper for the [ggwave](https://github.com/ggerganov/ggwave) library,
//! which enables data transmission via audio.
//!
//! This library provides a safe Rust interface to the C ggwave API, allowing
//! for audio-based data transmission and reception using various protocols.
//!
//! ## Features
//!
//! - Encode text into audio waveforms
//! - Decode audio waveforms back into text
//! - Support for various protocols (audible, ultrasound, etc.)
//! - Zero-copy API options for performance
//! - Customizable parameters for transmission
//! - Export encoded audio to WAV format
//!
//! ## Example
//!
//! ```rust
//! use ggwave_rs::{GGWave, protocols};
//!
//! // Create a new GGWave instance with default parameters
//! let ggwave = GGWave::new().expect("Failed to initialize GGWave");
//!
//! // Encode text using audible protocol with volume 50
//! let waveform = ggwave.encode("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
//!     .expect("Failed to encode text");
//!
//! // Save to WAV file
//! ggwave.save_raw_to_wav(&waveform, "hello.wav")
//!     .expect("Failed to save WAV file");
//!
//! // Decode waveform (in a real application, this would be captured from a microphone)
//! let mut buffer = vec![0u8; 1024];
//! let decoded = ggwave.decode(&waveform, &mut buffer)
//!     .expect("Failed to decode waveform");
//!
//! assert_eq!(decoded, "Hello, World!");
//! ```

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::c_void;
use std::io::Cursor;
use std::path::Path;
use std::ptr;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use ffi::constants;
use hound::{WavSpec, WavWriter};

// Static initialization
static INIT: Once = Once::new();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

//
// Public types
//

pub use ggwave_Filter as Filter;
pub use ggwave_Parameters as Parameters;
pub use ggwave_ProtocolId as ProtocolId;
pub use ggwave_SampleFormat as SampleFormat;

/// Raw FFI bindings to the ggwave C API
///
/// # Safety
///
/// These functions are unsafe and require proper understanding of the underlying C API.
/// Use the safe wrapper functions provided by the `GGWave` struct when possible.
pub mod ffi;

#[cfg(feature = "async")]
pub mod async_impl;

/// Error type for ggwave operations
#[derive(Debug)]
pub enum Error {
    /// Encoding failed with specific error code
    EncodeFailed(i32),
    /// Decoding failed with specific error code
    DecodeFailed(i32),
    /// Failed to write WAV file
    WavWriteFailed(hound::Error),
    /// Invalid sample format
    InvalidSampleFormat,
    /// I/O error
    IoError(std::io::Error),
    /// UTF-8 conversion error
    Utf8Error(std::str::Utf8Error),
    /// Invalid parameter
    InvalidParameter(&'static str),
    /// Initialization failed
    InitializationFailed,
    /// Buffer too small
    BufferTooSmall { required: usize, provided: usize },
    /// Text too long for encoding
    TextTooLong { length: usize, max: usize },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EncodeFailed(code) => write!(f, "Failed to encode data, error code: {}", code),
            Error::DecodeFailed(code) => write!(f, "Failed to decode data, error code: {}", code),
            Error::WavWriteFailed(e) => write!(f, "WAV write error: {}", e),
            Error::InvalidSampleFormat => write!(f, "Invalid sample format"),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::Utf8Error(e) => write!(f, "UTF-8 conversion error: {}", e),
            Error::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            Error::InitializationFailed => write!(f, "Failed to initialize GGWave"),
            Error::BufferTooSmall { required, provided } => write!(
                f,
                "Buffer too small, required: {} bytes, provided: {} bytes",
                required, provided
            ),
            Error::TextTooLong { length, max } => write!(
                f,
                "Text too long for encoding, length: {} bytes, max: {} bytes",
                length, max
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<hound::Error> for Error {
    fn from(err: hound::Error) -> Self {
        Error::WavWriteFailed(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::Utf8Error(err)
    }
}

/// Result type for ggwave operations
pub type Result<T> = std::result::Result<T, Error>;

/// Builder for GGWave parameters
///
/// This struct allows for configuring a GGWave instance in a fluent manner.
pub struct GGWaveBuilder {
    params: Parameters,
}

impl GGWaveBuilder {
    /// Create a new builder with modified default parameters
    ///
    /// Uses parameter values that reliably work across different systems.
    pub fn new() -> Self {
        let mut params = unsafe { ggwave_getDefaultParameters() };

        // Set sensible defaults that are known to work
        params.sampleRate = 16000.0;
        params.sampleRateInp = 16000.0;
        params.sampleRateOut = 16000.0;
        params.samplesPerFrame = 512;
        params.soundMarkerThreshold = 0.5;

        Self { params }
    }

    /// Set the sample rate for input, output, and processing
    pub fn sample_rate(mut self, rate: f32) -> Self {
        self.params.sampleRate = rate;
        self.params.sampleRateInp = rate;
        self.params.sampleRateOut = rate;
        self
    }

    /// Set the input sample rate
    pub fn input_sample_rate(mut self, rate: f32) -> Self {
        self.params.sampleRateInp = rate;
        self
    }

    /// Set the output sample rate
    pub fn output_sample_rate(mut self, rate: f32) -> Self {
        self.params.sampleRateOut = rate;
        self
    }

    /// Set samples per frame
    pub fn samples_per_frame(mut self, samples: i32) -> Self {
        self.params.samplesPerFrame = samples;
        self
    }

    /// Set input sample format
    pub fn input_sample_format(mut self, format: SampleFormat) -> Self {
        self.params.sampleFormatInp = format;
        self
    }

    /// Set output sample format
    pub fn output_sample_format(mut self, format: SampleFormat) -> Self {
        self.params.sampleFormatOut = format;
        self
    }

    /// Set sound marker threshold
    pub fn sound_marker_threshold(mut self, threshold: f32) -> Self {
        self.params.soundMarkerThreshold = threshold;
        self
    }

    /// Set operating mode
    pub fn operating_mode(mut self, mode: i32) -> Self {
        self.params.operatingMode = mode;
        self
    }

    /// Set fixed payload length
    pub fn fixed_payload_length(mut self, length: i32) -> Self {
        if length <= 0 || length > constants::MAX_LENGTH_FIXED as i32 {
            panic!(
                "Fixed payload length must be between 1 and {}",
                constants::MAX_LENGTH_FIXED
            );
        }
        self.params.payloadLength = length;
        self
    }

    /// Build a GGWave instance with the configured parameters
    pub fn build(self) -> Result<GGWave> {
        unsafe {
            let instance = ggwave_init(self.params);
            if instance < 0 {
                Err(Error::InitializationFailed)
            } else {
                Ok(GGWave { instance })
            }
        }
    }
}

impl Default for GGWaveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Main GGWave interface for audio-based data transmission
///
/// This struct provides a safe interface to the ggwave C API, allowing for
/// encoding and decoding of data using audio.
pub struct GGWave {
    instance: ggwave_Instance,
}

impl GGWave {
    /// Get the raw ggwave instance handle for advanced use cases
    ///
    /// # Safety
    ///
    /// This function returns the raw instance handle which can be used with the unsafe functions
    /// in the `ffi` module. Using this handle improperly can lead to undefined behavior.
    pub fn raw_instance(&self) -> ffi::ggwave_Instance {
        self.instance
    }

    /// Execute a custom operation with the raw ggwave instance
    ///
    /// # Safety
    ///
    /// The provided function `f` must use the instance safely according to the ggwave C API.
    pub unsafe fn with_raw_instance<F, T>(&self, f: F) -> T
    where
        F: FnOnce(ffi::ggwave_Instance) -> T,
    {
        f(self.instance)
    }

    /// Create a GGWave instance from an existing raw instance
    ///
    /// # Safety
    ///
    /// The provided instance must be a valid ggwave instance created with `ggwave_init`.
    /// The instance will be owned by the returned GGWave and will be freed when dropped.
    pub unsafe fn from_raw_instance(instance: ffi::ggwave_Instance) -> Self {
        if instance < 0 {
            panic!("Invalid ggwave instance");
        }
        Self { instance }
    }

    /// Create a new GGWave instance with modified default parameters
    ///
    /// Uses parameter values that reliably work across different systems.
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::GGWave;
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// ```
    pub fn new() -> Result<Self> {
        // Initialize global state if needed
        INIT.call_once(|| {
            // Any global initialization can go here
            INITIALIZED.store(true, Ordering::SeqCst);
        });

        unsafe {
            // Start with default parameters
            let params = ggwave_getDefaultParameters();

            // Initialize with modified parameters
            let instance = ggwave_init(params);
            if instance < 0 {
                Err(Error::InitializationFailed)
            } else {
                Ok(Self { instance })
            }
        }
    }

    /// Start building a GGWave instance with custom parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, sample_formats};
    ///
    /// let ggwave = GGWave::builder()
    ///     .sample_rate(48000.0)
    ///     .output_sample_format(sample_formats::F32)
    ///     .build()
    ///     .expect("Failed to initialize GGWave");
    /// ```
    pub fn builder() -> GGWaveBuilder {
        GGWaveBuilder::new()
    }

    /// Create a new GGWave instance with fixed-length encoding
    ///
    /// # Arguments
    ///
    /// * `payload_length` - Fixed payload length to use (must be <= 64)
    /// * `operating_mode` - Operating mode to use (default: RX_AND_TX)
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, operating_modes};
    ///
    /// // Create instance with 64-byte fixed payload length
    /// let ggwave = GGWave::new_with_fixed_payload(64, operating_modes::RX_AND_TX)
    ///     .expect("Failed to initialize GGWave");
    /// ```
    pub fn new_with_fixed_payload(payload_length: i32, operating_mode: i32) -> Result<Self> {
        if payload_length <= 0 || payload_length > constants::MAX_LENGTH_FIXED as i32 {
            return Err(Error::InvalidParameter(
                "Fixed payload length must be between 1 and 64",
            ));
        }

        unsafe {
            let mut params = ggwave_getDefaultParameters();
            params.payloadLength = payload_length;
            params.operatingMode = operating_mode;
            let instance = ggwave_init(params);
            if instance < 0 {
                Err(Error::InitializationFailed)
            } else {
                Ok(Self { instance })
            }
        }
    }

    /// Create a new GGWave instance with custom parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, Parameters, sample_formats};
    ///
    /// let mut params = GGWave::default_parameters();
    /// params.sampleFormatOut = sample_formats::F32;
    /// let ggwave = GGWave::new_with_params(params)
    ///     .expect("Failed to initialize GGWave");
    /// ```
    pub fn new_with_params(params: Parameters) -> Result<Self> {
        unsafe {
            let instance = ggwave_init(params);
            if instance < 0 {
                Err(Error::InitializationFailed)
            } else {
                Ok(Self { instance })
            }
        }
    }

    /// Get default parameters for ggwave
    ///
    /// # Returns
    ///
    /// A `Parameters` struct with default values
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::GGWave;
    ///
    /// let params = GGWave::default_parameters();
    /// println!("Default sample rate: {}", params.sampleRate);
    /// ```
    pub fn default_parameters() -> Parameters {
        unsafe { ggwave_getDefaultParameters() }
    }

    /// Check if the instance is configured for fixed-length payloads
    fn is_fixed_length(&self) -> bool {
        unsafe {
            let params = ggwave_getDefaultParameters();
            params.payloadLength > 0
        }
    }

    /// Calculate the required buffer size for encoding text
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    ///
    /// # Returns
    ///
    /// A `Result` containing the required buffer size in bytes
    pub fn calculate_encode_buffer_size(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
    ) -> Result<usize> {
        let max_length = if self.is_fixed_length() {
            unsafe { ggwave_getDefaultParameters().payloadLength as usize }
        } else {
            constants::MAX_LENGTH_VARIABLE
        };

        if text.len() > max_length {
            return Err(Error::TextTooLong {
                length: text.len(),
                max: max_length,
            });
        }

        unsafe {
            let payload_buffer = text.as_ptr() as *const c_void;
            let payload_size = text.len() as i32;

            let waveform_size = ggwave_encode(
                self.instance,
                payload_buffer,
                payload_size,
                protocol_id,
                volume,
                ptr::null_mut(),
                1, // query size in bytes
            );

            if waveform_size <= 0 {
                Err(Error::EncodeFailed(waveform_size))
            } else {
                Ok(waveform_size as usize)
            }
        }
    }

    /// Encode text into a provided buffer
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    /// * `buffer` - The buffer to encode into
    ///
    /// # Returns
    ///
    /// A `Result` containing the number of bytes written to the buffer
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// let text = "Hello, World!";
    ///
    /// // Calculate needed buffer size
    /// let size = ggwave.calculate_encode_buffer_size(text, protocols::AUDIBLE_NORMAL, 50)
    ///     .expect("Failed to calculate buffer size");
    ///
    /// // Allocate buffer
    /// let mut buffer = vec![0u8; size];
    ///
    /// // Encode into the buffer
    /// let bytes_written = ggwave.encode_into_buffer(text, protocols::AUDIBLE_NORMAL, 50, &mut buffer)
    ///     .expect("Failed to encode text");
    ///
    /// assert!(bytes_written <= buffer.len());
    /// ```
    pub fn encode_into_buffer(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
        buffer: &mut [u8],
    ) -> Result<usize> {
        let required_size = self.calculate_encode_buffer_size(text, protocol_id, volume)?;

        if buffer.len() < required_size {
            return Err(Error::BufferTooSmall {
                required: required_size,
                provided: buffer.len(),
            });
        }

        unsafe {
            let payload_buffer = text.as_ptr() as *const c_void;
            let payload_size = text.len() as i32;

            let result = ggwave_encode(
                self.instance,
                payload_buffer,
                payload_size,
                protocol_id,
                volume,
                buffer.as_mut_ptr() as *mut c_void,
                0, // perform actual encoding
            );

            if result < 0 {
                Err(Error::EncodeFailed(result))
            } else {
                Ok(result as usize)
            }
        }
    }

    /// Encode text to raw audio data with heap allocation
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` with the encoded audio data
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// let waveform = ggwave.encode("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
    ///     .expect("Failed to encode text");
    /// ```
    pub fn encode(&self, text: &str, protocol_id: ProtocolId, volume: i32) -> Result<Vec<u8>> {
        let size = self.calculate_encode_buffer_size(text, protocol_id, volume)?;
        let mut buffer = vec![0u8; size];
        let written = self.encode_into_buffer(text, protocol_id, volume, &mut buffer)?;

        // Trim the buffer to the actual size if needed
        if written < buffer.len() {
            buffer.truncate(written);
        }

        Ok(buffer)
    }

    /// Decode raw audio data to text using a provided buffer
    ///
    /// # Arguments
    ///
    /// * `waveform` - The raw audio data to decode
    /// * `buffer` - Buffer to store the decoded payload
    ///
    /// # Returns
    ///
    /// A `Result` containing the decoded text as a string slice
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// let waveform = ggwave.encode("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
    ///     .expect("Failed to encode text");
    ///
    /// let mut buffer = vec![0u8; 1024];
    /// let decoded = ggwave.decode(&waveform, &mut buffer)
    ///     .expect("Failed to decode waveform");
    ///
    /// assert_eq!(decoded, "Hello, World!");
    /// ```
    pub fn decode<'a>(&self, waveform: &[u8], buffer: &'a mut [u8]) -> Result<&'a str> {
        unsafe {
            let waveform_buffer = waveform.as_ptr() as *const c_void;
            let waveform_size = waveform.len() as i32;

            let result = ggwave_ndecode(
                self.instance,
                waveform_buffer,
                waveform_size,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as i32,
            );

            if result < 0 {
                Err(Error::DecodeFailed(result))
            } else {
                // Return slice to valid data
                std::str::from_utf8(&buffer[..result as usize]).map_err(Error::Utf8Error)
            }
        }
    }

    /// Decode raw audio data to text with heap allocation
    ///
    /// # Arguments
    ///
    /// * `waveform` - The raw audio data to decode
    /// * `max_payload_size` - The maximum size of the decoded payload
    ///
    /// # Returns
    ///
    /// A `Result` containing the decoded text
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// let waveform = ggwave.encode("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
    ///     .expect("Failed to encode text");
    ///
    /// let decoded = ggwave.decode_to_string(&waveform, 1024)
    ///     .expect("Failed to decode waveform");
    ///
    /// assert_eq!(decoded, "Hello, World!");
    /// ```
    pub fn decode_to_string(&self, waveform: &[u8], max_payload_size: usize) -> Result<String> {
        let mut buffer = vec![0u8; max_payload_size];
        let decoded = self.decode(waveform, &mut buffer)?;
        Ok(decoded.to_string())
    }

    /// Get the current output sample format
    ///
    /// # Returns
    ///
    /// The current output sample format
    pub fn get_output_sample_format(&self) -> SampleFormat {
        unsafe { ggwave_getDefaultParameters().sampleFormatOut }
    }

    /// Convert raw audio data to WAV format in memory
    ///
    /// # Arguments
    ///
    /// * `raw_data` - The raw audio data to convert
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` with the WAV data
    pub fn raw_to_wav(&self, raw_data: &[u8]) -> Result<Vec<u8>> {
        let params = unsafe { ggwave_getDefaultParameters() };
        let sample_rate = params.sampleRateOut as u32;
        let format = params.sampleFormatOut;

        // Create WAV spec
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut buffer = Vec::new();
        let mut writer =
            WavWriter::new(Cursor::new(&mut buffer), spec).map_err(Error::WavWriteFailed)?;

        match format {
            // Float32 format
            ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_F32 => {
                let samples = unsafe {
                    std::slice::from_raw_parts(
                        raw_data.as_ptr() as *const f32,
                        raw_data.len() / std::mem::size_of::<f32>(),
                    )
                };

                for &sample in samples {
                    let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    writer.write_sample(sample_i16)?;
                }
            }
            // Int16 format
            ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I16 => {
                let samples = unsafe {
                    std::slice::from_raw_parts(
                        raw_data.as_ptr() as *const i16,
                        raw_data.len() / std::mem::size_of::<i16>(),
                    )
                };

                for &sample in samples {
                    writer.write_sample(sample)?;
                }
            }
            // Other formats (best effort)
            _ => {
                let samples = unsafe {
                    std::slice::from_raw_parts(raw_data.as_ptr() as *const i16, raw_data.len() / 2)
                };

                for &sample in samples {
                    writer.write_sample(sample)?;
                }
            }
        }

        writer.finalize()?;
        Ok(buffer)
    }

    /// Encode text and convert to WAV format
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec<u8>` with the WAV data
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    /// use std::fs;
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// let wav_data = ggwave.encode_to_wav("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
    ///     .expect("Failed to encode text to WAV");
    ///
    /// fs::write("hello.wav", wav_data).expect("Failed to write WAV file");
    /// ```
    pub fn encode_to_wav(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
    ) -> Result<Vec<u8>> {
        let raw_data = self.encode(text, protocol_id, volume)?;
        self.raw_to_wav(&raw_data)
    }

    /// Save raw audio data to a WAV file
    ///
    /// # Arguments
    ///
    /// * `raw_data` - The raw audio data to save
    /// * `path` - The path to save the WAV file to
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    pub fn save_raw_to_wav<P: AsRef<Path>>(&self, raw_data: &[u8], path: P) -> Result<()> {
        let wav_data = self.raw_to_wav(raw_data)?;
        std::fs::write(path, wav_data)?;
        Ok(())
    }

    /// Encode text and save directly to a WAV file
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    /// * `path` - The path to save the WAV file to
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// ggwave.encode_to_wav_file("Hello, World!", protocols::AUDIBLE_NORMAL, 50, "hello.wav")
    ///     .expect("Failed to encode and save WAV file");
    /// ```
    pub fn encode_to_wav_file<P: AsRef<Path>>(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
        path: P,
    ) -> Result<()> {
        let raw_data = self.encode(text, protocol_id, volume)?;
        self.save_raw_to_wav(&raw_data, path)
    }

    /// Toggle reception of a specific protocol
    ///
    /// # Arguments
    ///
    /// * `protocol_id` - The protocol to toggle
    /// * `enabled` - Whether to enable or disable the protocol
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::{GGWave, protocols};
    ///
    /// let ggwave = GGWave::new().expect("Failed to initialize GGWave");
    /// // Disable reception of ultrasound protocols
    /// ggwave.toggle_rx_protocol(protocols::ULTRASOUND_NORMAL, false);
    /// ggwave.toggle_rx_protocol(protocols::ULTRASOUND_FAST, false);
    /// ggwave.toggle_rx_protocol(protocols::ULTRASOUND_FASTEST, false);
    /// ```
    pub fn toggle_rx_protocol(&self, protocol_id: ProtocolId, enabled: bool) {
        unsafe {
            ggwave_rxToggleProtocol(protocol_id, if enabled { 1 } else { 0 });
        }
    }

    /// Toggle transmission of a specific protocol
    ///
    /// # Arguments
    ///
    /// * `protocol_id` - The protocol to toggle
    /// * `enabled` - Whether to enable or disable the protocol
    pub fn toggle_tx_protocol(&self, protocol_id: ProtocolId, enabled: bool) {
        unsafe {
            ggwave_txToggleProtocol(protocol_id, if enabled { 1 } else { 0 });
        }
    }

    /// Set the starting frequency for a reception protocol
    ///
    /// # Arguments
    ///
    /// * `protocol_id` - The protocol to modify
    /// * `freq_start` - The starting frequency in Hz
    pub fn set_rx_protocol_freq_start(&self, protocol_id: ProtocolId, freq_start: i32) {
        unsafe {
            ggwave_rxProtocolSetFreqStart(protocol_id, freq_start);
        }
    }

    /// Set the starting frequency for a transmission protocol
    ///
    /// # Arguments
    ///
    /// * `protocol_id` - The protocol to modify
    /// * `freq_start` - The starting frequency in Hz
    pub fn set_tx_protocol_freq_start(&self, protocol_id: ProtocolId, freq_start: i32) {
        unsafe {
            ggwave_txProtocolSetFreqStart(protocol_id, freq_start);
        }
    }

    /// Get the duration in frames for reception
    ///
    /// # Returns
    ///
    /// The duration in frames
    pub fn rx_duration_frames(&self) -> i32 {
        unsafe { ggwave_rxDurationFrames(self.instance) }
    }

    /// Set debug mode and optionally redirect logs to a file
    ///
    /// # Arguments
    ///
    /// * `debug_file` - Optional path to a log file, or None to disable logging
    ///
    /// # Safety
    ///
    /// This function is marked safe but internally uses unsafe operations to interact
    /// with C file handling. The file path must be valid and accessible.
    pub fn set_debug_mode(&self, debug_file: Option<&str>) {
        unsafe {
            match debug_file {
                Some(path) => {
                    // Try to open the file in C
                    let c_str = std::ffi::CString::new(path).unwrap();
                    let mode = std::ffi::CString::new("w").unwrap();
                    let file_ptr = libc::fopen(c_str.as_ptr(), mode.as_ptr());
                    if !file_ptr.is_null() {
                        ggwave_setLogFile(file_ptr as *mut c_void);
                    }
                }
                None => {
                    // Disable logging
                    ggwave_setLogFile(std::ptr::null_mut());
                }
            }
        }
    }

    /// Enables all reception protocols
    ///
    /// This is a convenience method to enable all available protocols for reception.
    pub fn enable_all_rx_protocols(&self) {
        for protocol_id in 0..protocols::COUNT {
            self.toggle_rx_protocol(protocol_id, true);
        }
    }

    /// Decode raw audio data to binary data
    ///
    /// This variant of decode is useful when the data being transmitted is not UTF-8 text.
    ///
    /// # Arguments
    ///
    /// * `waveform` - The raw audio data to decode
    /// * `buffer` - Buffer to store the decoded payload
    ///
    /// # Returns
    ///
    /// A `Result` containing a slice of the decoded binary data
    pub fn decode_binary<'a>(&self, waveform: &[u8], buffer: &'a mut [u8]) -> Result<&'a [u8]> {
        unsafe {
            let result = ggwave_ndecode(
                self.instance,
                waveform.as_ptr() as *const c_void,
                waveform.len() as i32,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as i32,
            );

            if result < 0 {
                Err(Error::DecodeFailed(result))
            } else {
                Ok(&buffer[..result as usize])
            }
        }
    }

    /// Memory-efficient continuous audio decoder
    ///
    /// This method is designed for real-time continuous audio processing where
    /// audio is being processed in chunks, such as from a microphone input.
    ///
    /// # Arguments
    ///
    /// * `audio_chunk` - New chunk of audio data to process
    /// * `decode_buffer` - Buffer to store decoded payload if found
    ///
    /// # Returns
    ///
    /// A `Result` containing an Option with the decoded string if something was found
    pub fn process_audio_chunk<'a>(
        &self,
        audio_chunk: &[u8],
        decode_buffer: &'a mut [u8],
    ) -> Result<Option<&'a str>> {
        unsafe {
            let result = ggwave_decode(
                self.instance,
                audio_chunk.as_ptr() as *const c_void,
                audio_chunk.len() as i32,
                decode_buffer.as_mut_ptr() as *mut c_void,
            );

            if result < 0 {
                // No data found or error
                if result < 0 {
                    Err(Error::DecodeFailed(result))
                } else {
                    Ok(None) // No data decoded, but no error
                }
            } else {
                // Something was decoded
                match std::str::from_utf8(&decode_buffer[..result as usize]) {
                    Ok(s) => Ok(Some(s)),
                    Err(e) => Err(Error::Utf8Error(e)),
                }
            }
        }
    }

    /// Estimate the duration of the encoded audio in seconds
    ///
    /// # Arguments
    ///
    /// * `protocol_id` - The protocol used for encoding
    /// * `text_length` - The length of the text in bytes
    ///
    /// # Returns
    ///
    /// Estimated duration in seconds
    pub fn estimate_duration(&self, protocol_id: ProtocolId, text_length: usize) -> f32 {
        // This is an approximation based on the protocol and text length
        // Get parameters from the instance, but we don't need the sample rate for this calculation
        unsafe { ggwave_getDefaultParameters() };

        // Each protocol has different timing characteristics
        let seconds_per_byte = match protocol_id {
            id if id == protocols::AUDIBLE_FASTEST
                || id == protocols::ULTRASOUND_FASTEST
                || id == protocols::DT_FASTEST
                || id == protocols::MT_FASTEST =>
            {
                0.01
            }

            id if id == protocols::AUDIBLE_FAST
                || id == protocols::ULTRASOUND_FAST
                || id == protocols::DT_FAST
                || id == protocols::MT_FAST =>
            {
                0.02
            }

            _ => 0.03, // Normal speed protocols
        };

        // Base duration plus per-byte duration
        let base_duration = 0.2; // Protocol overhead
        base_duration + (text_length as f32 * seconds_per_byte)
    }
}

impl Default for GGWave {
    fn default() -> Self {
        Self::new().expect("Failed to initialize GGWave with default parameters")
    }
}

impl Drop for GGWave {
    fn drop(&mut self) {
        unsafe {
            ggwave_free(self.instance);
        }
    }
}

/// Protocol constants module for easier import
///
/// This module provides constants for all the available transmission protocols.
pub mod protocols {
    use super::*;

    /// Standard audible protocol with normal speed
    pub const AUDIBLE_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_NORMAL;
    /// Fast audible protocol
    pub const AUDIBLE_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FAST;
    /// Fastest audible protocol
    pub const AUDIBLE_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FASTEST;

    /// Standard ultrasound protocol with normal speed
    pub const ULTRASOUND_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_NORMAL;
    /// Fast ultrasound protocol
    pub const ULTRASOUND_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FAST;
    /// Fastest ultrasound protocol
    pub const ULTRASOUND_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FASTEST;

    /// DT protocol with normal speed
    pub const DT_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_NORMAL;
    /// Fast DT protocol
    pub const DT_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_FAST;
    /// Fastest DT protocol
    pub const DT_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_FASTEST;

    /// MT protocol with normal speed
    pub const MT_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_NORMAL;
    /// Fast MT protocol
    pub const MT_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_FAST;
    /// Fastest MT protocol
    pub const MT_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_FASTEST;

    /// Custom protocol 0
    pub const CUSTOM_0: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_0;
    /// Custom protocol 1
    pub const CUSTOM_1: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_1;
    /// Custom protocol 2
    pub const CUSTOM_2: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_2;
    /// Custom protocol 3
    pub const CUSTOM_3: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_3;
    /// Custom protocol 4
    pub const CUSTOM_4: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_4;
    /// Custom protocol 5
    pub const CUSTOM_5: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_5;
    /// Custom protocol 6
    pub const CUSTOM_6: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_6;
    /// Custom protocol 7
    pub const CUSTOM_7: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_7;
    /// Custom protocol 8
    pub const CUSTOM_8: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_8;
    /// Custom protocol 9
    pub const CUSTOM_9: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_9;
    /// Total number of protocols
    pub const COUNT: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_COUNT;
}

/// Sample format constants
///
/// This module provides constants for all the available sample formats.
pub mod sample_formats {
    use super::*;

    /// Undefined sample format
    pub const UNDEFINED: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_UNDEFINED;
    /// Unsigned 8-bit sample format
    pub const U8: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U8;
    /// Signed 8-bit sample format
    pub const I8: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I8;
    /// Unsigned 16-bit sample format
    pub const U16: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U16;
    /// Signed 16-bit sample format
    pub const I16: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I16;
    /// 32-bit float sample format
    pub const F32: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_F32;
}

/// Operating mode constants
///
/// This module provides constants for all the available operating modes.
pub mod operating_modes {
    use super::*;

    /// Reception mode
    pub const RX: i32 = GGWAVE_OPERATING_MODE_RX as i32;
    /// Transmission mode
    pub const TX: i32 = GGWAVE_OPERATING_MODE_TX as i32;
    /// Reception and transmission mode
    pub const RX_AND_TX: i32 = GGWAVE_OPERATING_MODE_RX_AND_TX as i32;
    /// Transmission of tones only
    pub const TX_ONLY_TONES: i32 = GGWAVE_OPERATING_MODE_TX_ONLY_TONES as i32;
    /// Use DSS (Direct Sequence Spread)
    pub const USE_DSS: i32 = GGWAVE_OPERATING_MODE_USE_DSS as i32;
}

/// Filter type constants
///
/// This module provides constants for all the available filter types.
pub mod filters {
    use super::*;

    /// Hann window filter
    pub const HANN: Filter = ggwave_Filter_GGWAVE_FILTER_HANN;
    /// Hamming window filter
    pub const HAMMING: Filter = ggwave_Filter_GGWAVE_FILTER_HAMMING;
    /// First order high-pass filter
    pub const FIRST_ORDER_HIGH_PASS: Filter = ggwave_Filter_GGWAVE_FILTER_FIRST_ORDER_HIGH_PASS;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let ggwave = GGWave::new().expect("Failed to initialize GGWave");
        drop(ggwave);
    }

    #[test]
    fn test_encode_decode() {
        let ggwave = GGWave::new().expect("Failed to initialize GGWave");
        let text = "Hello, GGWave!";

        let waveform = ggwave
            .encode(text, protocols::AUDIBLE_NORMAL, 50)
            .expect("Failed to encode text");

        let mut buffer = vec![0u8; 1024];
        let decoded = ggwave
            .decode(&waveform, &mut buffer)
            .expect("Failed to decode waveform");

        assert_eq!(decoded, text);
    }

    #[test]
    fn test_builder() {
        let ggwave = GGWave::builder()
            .sample_rate(48000.0)
            .output_sample_format(sample_formats::F32)
            .build()
            .expect("Failed to initialize GGWave with builder");

        let format = ggwave.get_output_sample_format();
        assert_eq!(format, sample_formats::F32);
    }

    #[test]
    fn test_encode_into_buffer() {
        let ggwave = GGWave::new().expect("Failed to initialize GGWave");
        let text = "Test buffer encode";

        let size = ggwave
            .calculate_encode_buffer_size(text, protocols::AUDIBLE_NORMAL, 50)
            .expect("Failed to calculate buffer size");

        let mut buffer = vec![0u8; size];
        let written = ggwave
            .encode_into_buffer(text, protocols::AUDIBLE_NORMAL, 50, &mut buffer)
            .expect("Failed to encode text into buffer");

        assert!(written > 0);
        assert!(written <= buffer.len());
    }

    #[test]
    fn test_decode_binary() {
        let ggwave = GGWave::new().expect("Failed to initialize GGWave");
        let data = [1u8, 2, 3, 4, 5];

        // First encode the binary data as a string
        let encoded = ggwave
            .encode(
                &String::from_utf8_lossy(&data),
                protocols::AUDIBLE_FASTEST,
                50,
            )
            .expect("Failed to encode binary data");

        // Then decode back to binary
        let mut buffer = vec![0u8; 1024];
        let decoded = ggwave
            .decode_binary(&encoded, &mut buffer)
            .expect("Failed to decode binary data");

        assert_eq!(decoded, data);
    }
}
