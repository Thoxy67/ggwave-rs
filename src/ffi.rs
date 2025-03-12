//! Raw FFI bindings to the ggwave C API
//!
//! This module exposes the raw C API functions and types for ggwave.
//! Most users should prefer the safe wrapper functions provided by the `GGWave` struct.
//!
//! # Safety
//!
//! All functions in this module are unsafe and require proper understanding of the
//! underlying C API and memory management. Use with caution.

pub mod constants {
    /// Maximum data size for decoding buffer
    pub const MAX_DATA_SIZE: usize = 256;

    /// Maximum length for variable-length encoding
    pub const MAX_LENGTH_VARIABLE: usize = 140;

    /// Maximum length for fixed-length encoding
    pub const MAX_LENGTH_FIXED: usize = 64;

    /// Default number of marker frames
    pub const DEFAULT_MARKER_FRAMES: usize = 16;

    /// Default encoded data offset
    pub const DEFAULT_ENCODED_DATA_OFFSET: usize = 3;
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
    ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FAST,
    ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_FASTEST,
    // Constants - Protocol IDs
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
    ggwave_SampleFormat,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_F32,

    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I8,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_I16,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U8,
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_U16,
    // Constants - Sample formats
    ggwave_SampleFormat_GGWAVE_SAMPLE_FORMAT_UNDEFINED,
    ggwave_decode,
    ggwave_encode,
    ggwave_free,
    ggwave_getDefaultParameters,
    ggwave_init,
    ggwave_ndecode,
    ggwave_rxDurationFrames,
    ggwave_rxProtocolSetFreqStart,
    ggwave_rxToggleProtocol,
    // Functions
    ggwave_setLogFile,
    ggwave_txProtocolSetFreqStart,
    ggwave_txToggleProtocol,
};
