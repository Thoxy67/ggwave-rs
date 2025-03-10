#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Include the generated bindings
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::c_void;
use std::io::Cursor;
use std::path::Path;
use std::ptr;

use hound::{WavSpec, WavWriter};

//
// Public types
//

pub use ggwave_SampleFormat as SampleFormat;
pub use ggwave_ProtocolId as ProtocolId;
pub use ggwave_Filter as Filter;
pub use ggwave_Parameters as Parameters;

/// Raw FFI bindings to the ggwave C API
/// 
/// # Safety
/// 
/// These functions are unsafe and require proper understanding of the underlying C API.
/// Use the safe wrapper functions provided by the `GGWave` struct when possible.
pub mod ffi;

/// Error type for ggwave operations
#[derive(Debug)]
pub enum Error {
    /// Encoding failed
    EncodeFailed,
    /// Decoding failed
    DecodeFailed,
    /// Failed to write WAV file
    WavWriteFailed(hound::Error),
    /// Invalid sample format
    InvalidSampleFormat,
    /// I/O error
    IoError(std::io::Error),
    /// UTF-8 conversion error
    Utf8Error(std::string::FromUtf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EncodeFailed => write!(f, "Failed to encode data"),
            Error::DecodeFailed => write!(f, "Failed to decode data"),
            Error::WavWriteFailed(e) => write!(f, "WAV write error: {}", e),
            Error::InvalidSampleFormat => write!(f, "Invalid sample format"),
            Error::IoError(e) => write!(f, "IO error: {}", e),
            Error::Utf8Error(e) => write!(f, "UTF-8 conversion error: {}", e),
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

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Error::Utf8Error(err)
    }
}

/// Result type for ggwave operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main GGWave interface for audio-based data transmission
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
        F: FnOnce(ffi::ggwave_Instance) -> T
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
        Self { instance }
    }

    /// Create a new GGWave instance with default parameters
    pub fn new() -> Self {
        unsafe {
            let params = ggwave_getDefaultParameters();
            let instance = ggwave_init(params);
            Self { instance }
        }
    }
    
    /// Create a new GGWave instance with custom parameters
    pub fn new_with_params(params: Parameters) -> Self {
        unsafe {
            let instance = ggwave_init(params);
            Self { instance }
        }
    }
    
    /// Get default parameters
    pub fn default_parameters() -> Parameters {
        unsafe { ggwave_getDefaultParameters() }
    }
    
    /// Encode text to raw audio data
    pub fn encode(&self, text: &str, protocol_id: ProtocolId, volume: i32) -> Result<Vec<u8>> {
        unsafe {
            let payload_buffer = text.as_ptr() as *const c_void;
            let payload_size = text.len() as i32;
            
            // First call to determine the required buffer size
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
                return Err(Error::EncodeFailed);
            }
            
            // Allocate buffer for the encoded waveform
            let mut waveform_buffer = vec![0u8; waveform_size as usize];
            
            // Second call to actually encode
            let result = ggwave_encode(
                self.instance,
                payload_buffer,
                payload_size,
                protocol_id,
                volume,
                waveform_buffer.as_mut_ptr() as *mut c_void,
                0, // perform actual encoding
            );
            
            if result <= 0 {
                Err(Error::EncodeFailed)
            } else {
                Ok(waveform_buffer)
            }
        }
    }
    
    /// Decode raw audio data to text
    pub fn decode(&self, waveform: &[u8], max_payload_size: usize) -> Result<String> {
        unsafe {
            let mut payload_buffer = vec![0u8; max_payload_size];
            
            let waveform_buffer = waveform.as_ptr() as *const c_void;
            let waveform_size = waveform.len() as i32;
            
            let result = ggwave_ndecode(
                self.instance,
                waveform_buffer,
                waveform_size,
                payload_buffer.as_mut_ptr() as *mut c_void,
                payload_buffer.len() as i32,
            );
            
            if result <= 0 {
                Err(Error::DecodeFailed)
            } else {
                // Truncate to actual size and convert to String
                payload_buffer.truncate(result as usize);
                Ok(String::from_utf8(payload_buffer)?)
            }
        }
    }

    /// Get the current output sample format
    pub fn get_output_sample_format(&self) -> SampleFormat {
        unsafe { ggwave_getDefaultParameters().sampleFormatOut }
    }

    /// Convert raw audio data to WAV format in memory
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
        let mut writer = WavWriter::new(Cursor::new(&mut buffer), spec)
            .map_err(Error::WavWriteFailed)?;
        
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
            },
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
            },
            // Other formats (best effort)
            _ => {
                let samples = unsafe {
                    std::slice::from_raw_parts(
                        raw_data.as_ptr() as *const i16,
                        raw_data.len() / 2,
                    )
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
    pub fn encode_to_wav(&self, text: &str, protocol_id: ProtocolId, volume: i32) -> Result<Vec<u8>> {
        let raw_data = self.encode(text, protocol_id, volume)?;
        self.raw_to_wav(&raw_data)
    }
    
    /// Save raw audio data to a WAV file
    pub fn save_raw_to_wav<P: AsRef<Path>>(&self, raw_data: &[u8], path: P) -> Result<()> {
        let wav_data = self.raw_to_wav(raw_data)?;
        std::fs::write(path, wav_data)?;
        Ok(())
    }
    
    /// Encode text and save directly to a WAV file
    pub fn encode_to_wav_file<P: AsRef<Path>>(
        &self, 
        text: &str, 
        protocol_id: ProtocolId, 
        volume: i32, 
        path: P
    ) -> Result<()> {
        let raw_data = self.encode(text, protocol_id, volume)?;
        self.save_raw_to_wav(&raw_data, path)
    }
    
    /// Toggle reception of a specific protocol
    pub fn toggle_rx_protocol(&self, protocol_id: ProtocolId, enabled: bool) {
        unsafe {
            ggwave_rxToggleProtocol(protocol_id, if enabled { 1 } else { 0 });
        }
    }
    
    /// Toggle transmission of a specific protocol
    pub fn toggle_tx_protocol(&self, protocol_id: ProtocolId, enabled: bool) {
        unsafe {
            ggwave_txToggleProtocol(protocol_id, if enabled { 1 } else { 0 });
        }
    }
    
    /// Set the starting frequency for a reception protocol
    pub fn set_rx_protocol_freq_start(&self, protocol_id: ProtocolId, freq_start: i32) {
        unsafe {
            ggwave_rxProtocolSetFreqStart(protocol_id, freq_start);
        }
    }
    
    /// Set the starting frequency for a transmission protocol
    pub fn set_tx_protocol_freq_start(&self, protocol_id: ProtocolId, freq_start: i32) {
        unsafe {
            ggwave_txProtocolSetFreqStart(protocol_id, freq_start);
        }
    }
    
    /// Get the duration in frames for reception
    pub fn rx_duration_frames(&self) -> i32 {
        unsafe {
            ggwave_rxDurationFrames(self.instance)
        }
    }

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
                },
                None => {
                    // Disable logging
                    ggwave_setLogFile(std::ptr::null_mut());
                }
            }
        }
    }

    pub fn decode_raw(&self, waveform: &[u8], max_payload_size: usize) -> Result<String> {
        unsafe {
            let mut payload_buffer = vec![0u8; max_payload_size];
            
            // Using decode instead of ndecode
            let result = ggwave_decode(
                self.instance,
                waveform.as_ptr() as *const c_void,
                waveform.len() as i32,
                payload_buffer.as_mut_ptr() as *mut c_void,
            );
            
            if result <= 0 {
                Err(Error::DecodeFailed)
            } else {
                // Truncate to actual size and convert to String
                payload_buffer.truncate(result as usize);
                match String::from_utf8(payload_buffer) {
                    Ok(s) => Ok(s),
                    Err(e) => Err(Error::Utf8Error(e))
                }
            }
        }
    }
}

impl Default for GGWave {
    fn default() -> Self {
        Self::new()
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
pub mod protocols {
    use super::*;
    
    // Audible protocols
    pub const AUDIBLE_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_NORMAL;
    pub const AUDIBLE_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FAST;
    pub const AUDIBLE_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FASTEST;
    
    // Ultrasound protocols
    pub const ULTRASOUND_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_NORMAL;
    pub const ULTRASOUND_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FAST;
    pub const ULTRASOUND_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FASTEST;
    
    // DT protocols
    pub const DT_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_NORMAL;
    pub const DT_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_FAST;
    pub const DT_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_FASTEST;
    
    // MT protocols
    pub const MT_NORMAL: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_NORMAL;
    pub const MT_FAST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_FAST;
    pub const MT_FASTEST: ProtocolId = ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_FASTEST;
}

/// Sample format constants
pub mod sample_formats {
    use super::*;
    
    pub const UNDEFINED: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_UNDEFINED;
    pub const U8: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U8;
    pub const I8: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I8;
    pub const U16: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U16;
    pub const I16: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I16;
    pub const F32: SampleFormat = ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_F32;
}

/// Operating mode constants
pub mod operating_modes {
    use super::*;
    
    pub const RX: i32 = GGWAVE_OPERATING_MODE_RX as i32;
    pub const TX: i32 = GGWAVE_OPERATING_MODE_TX as i32;
    pub const RX_AND_TX: i32 = GGWAVE_OPERATING_MODE_RX_AND_TX as i32;
    pub const TX_ONLY_TONES: i32 = GGWAVE_OPERATING_MODE_TX_ONLY_TONES as i32;
    pub const USE_DSS: i32 = GGWAVE_OPERATING_MODE_USE_DSS as i32;
}

/// Filter type constants
pub mod filters {
    use super::*;
    
    pub const HANN: Filter = ggwave_Filter_GGWAVE_FILTER_HANN;
    pub const HAMMING: Filter = ggwave_Filter_GGWAVE_FILTER_HAMMING;
    pub const FIRST_ORDER_HIGH_PASS: Filter = ggwave_Filter_GGWAVE_FILTER_FIRST_ORDER_HIGH_PASS;
}