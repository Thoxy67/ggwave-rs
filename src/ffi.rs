//! Raw FFI bindings to the ggwave C API
//!
//! This module exposes the raw C API functions and types for ggwave.
//! Most users should prefer the safe wrapper functions provided by the `GGWave` struct.
//!
//! # Safety
//!
//! All functions in this module are unsafe and require proper understanding of the
//! underlying C API and memory management. Use with caution.

/// Constants for ggwave configuration and operation
pub mod constants {
    /// Maximum data size for decoding buffer in bytes
    pub const MAX_DATA_SIZE: usize = 256;

    /// Maximum length for variable-length encoding in bytes
    pub const MAX_LENGTH_VARIABLE: usize = 140;

    /// Maximum length for fixed-length encoding in bytes
    pub const MAX_LENGTH_FIXED: usize = 64;

    /// Default number of marker frames
    pub const DEFAULT_MARKER_FRAMES: usize = 16;

    /// Default encoded data offset
    pub const DEFAULT_ENCODED_DATA_OFFSET: usize = 3;

    /// Minimum allowed volume level (0-100)
    pub const MIN_VOLUME: i32 = 0;

    /// Maximum allowed volume level (0-100)
    pub const MAX_VOLUME: i32 = 100;

    /// Default volume level for encoding
    pub const DEFAULT_VOLUME: i32 = 50;

    /// Default sample rate for audio processing
    pub const DEFAULT_SAMPLE_RATE: f32 = 48000.0;

    /// Minimum recommended buffer size for decoding in bytes
    pub const MIN_DECODE_BUFFER_SIZE: usize = 1024;
}

/// Advanced options for configuring ggwave instances
pub mod options {
    /// Use interpolation for waveform generation
    ///
    /// This can improve audio quality at the cost of performance
    pub const USE_INTERPOLATION: u32 = 1 << 0;

    /// Use FFTW for FFT operations if available
    ///
    /// This can improve performance but requires FFTW to be available
    pub const USE_FFTW: u32 = 1 << 1;

    /// Use threading for parallel processing
    ///
    /// This can improve performance on multi-core systems
    pub const USE_THREADING: u32 = 1 << 2;
}

// Re-export all bindgen-generated items
pub use super::{
    _bindgen_ty_1,

    // Constants - Max instances
    GGWAVE_MAX_INSTANCES,

    // Constants - Operating modes
    GGWAVE_OPERATING_MODE_RX,
    GGWAVE_OPERATING_MODE_RX_AND_TX,
    GGWAVE_OPERATING_MODE_TX,
    GGWAVE_OPERATING_MODE_TX_ONLY_TONES,
    GGWAVE_OPERATING_MODE_USE_DSS,

    ggwave_Filter,
    ggwave_Filter_GGWAVE_FILTER_FIRST_ORDER_HIGH_PASS,
    ggwave_Filter_GGWAVE_FILTER_HAMMING,
    // Constants - Filters
    ggwave_Filter_GGWAVE_FILTER_HANN,

    // Types
    ggwave_Instance,
    ggwave_Parameters,
    ggwave_ProtocolId,

    // Protocol IDs
    ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FAST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FASTEST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_NORMAL,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_COUNT,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_0,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_1,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_2,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_3,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_4,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_5,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_6,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_7,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_8,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_CUSTOM_9,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_FAST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_FASTEST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_DT_NORMAL,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_FAST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_FASTEST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_MT_NORMAL,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FAST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FASTEST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_NORMAL,

    // Sample formats
    ggwave_SampleFormat,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_F32,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I8,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I16,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U8,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U16,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_UNDEFINED,

    // Functions
    ggwave_decode,
    ggwave_encode,
    ggwave_free,
    ggwave_getDefaultParameters,
    ggwave_init,
    ggwave_ndecode,
    ggwave_rxDurationFrames,
    ggwave_rxProtocolSetFreqStart,
    ggwave_rxToggleProtocol,
    ggwave_setLogFile,
    ggwave_txProtocolSetFreqStart,
    ggwave_txToggleProtocol,
};

/// Helper functions for working with ggwave parameters
/// Helper functions for working with ggwave parameters
pub mod helpers {
    use super::*;
    use std::ffi::c_void;

    /// Safely initialize ggwave with default parameters
    ///
    /// # Returns
    ///
    /// The ggwave instance or 0 if initialization failed
    pub unsafe fn init_default() -> ggwave_Instance {
        unsafe {
            let params = ggwave_getDefaultParameters();
            ggwave_init(params)
        }
    }

    /// Calculate required buffer size for encoding
    ///
    /// # Arguments
    ///
    /// * `instance` - The ggwave instance
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use
    /// * `volume` - The volume level (0-100)
    ///
    /// # Returns
    ///
    /// The required buffer size in bytes or a negative error code
    pub unsafe fn calculate_encode_size(
        instance: ggwave_Instance,
        text: &str,
        protocol_id: ggwave_ProtocolId,
        volume: i32,
    ) -> i32 {
        let payload_buffer = text.as_ptr() as *const c_void;
        let payload_size = text.len() as i32;

        unsafe {
            ggwave_encode(
                instance,
                payload_buffer,
                payload_size,
                protocol_id,
                volume,
                std::ptr::null_mut(),
                1, // query size in bytes
            )
        }
    }

    /// Check if a ggwave instance is valid
    ///
    /// # Arguments
    ///
    /// * `instance` - The ggwave instance to check
    ///
    /// # Returns
    ///
    /// `true` if the instance is valid, `false` otherwise
    pub unsafe fn is_valid_instance(instance: ggwave_Instance) -> bool {
        instance > 0 && instance <= GGWAVE_MAX_INSTANCES as i32
    }

    /// Get the sample rate for a ggwave protocol
    ///
    /// # Arguments
    ///
    /// * `protocol_id` - The protocol ID
    ///
    /// # Returns
    ///
    /// The recommended sample rate in Hz
    pub fn get_protocol_sample_rate(protocol_id: ggwave_ProtocolId) -> f32 {
        match protocol_id {
            id if id == ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_NORMAL
                || id == ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FAST
                || id == ggwave_ProtocolId_GGWAVE_PROTOCOL_ULTRASOUND_FASTEST =>
            {
                48000.0
            }
            _ => 44100.0, // Standard sample rate for other protocols
        }
    }

    /// Enable/disable multiple protocols at once
    ///
    /// # Arguments
    ///
    /// * `protocol_ids` - Array of protocol IDs to modify
    /// * `enabled` - Whether to enable or disable the protocols
    /// * `is_rx` - If true, modify reception protocols, otherwise transmission
    pub unsafe fn toggle_protocols(protocol_ids: &[ggwave_ProtocolId], enabled: bool, is_rx: bool) {
        let enabled_val = if enabled { 1 } else { 0 };

        for &protocol_id in protocol_ids {
            unsafe {
                if is_rx {
                    ggwave_rxToggleProtocol(protocol_id, enabled_val);
                } else {
                    ggwave_txToggleProtocol(protocol_id, enabled_val);
                }
            }
        }
    }
}
