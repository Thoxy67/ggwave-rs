//! Async implementation of ggwave for use with tokio
//!
//! This module provides async wrappers around the synchronous GGWave API,
//! allowing for non-blocking encode/decode operations and stream processing.

use crate::{Error, GGWave, Parameters, ProtocolId, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::task;

/// Async wrapper around GGWave
///
/// This struct provides an async interface to the GGWave functionality,
/// with methods that don't block the current task.
pub struct AsyncGGWave {
    /// Inner GGWave instance wrapped in an Arc<Mutex<>> for thread safety
    inner: Arc<Mutex<GGWave>>,
}

impl AsyncGGWave {
    /// Create a new AsyncGGWave instance with default parameters
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::async_impl::AsyncGGWave;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ggwave = AsyncGGWave::new().await.expect("Failed to initialize AsyncGGWave");
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        // Spawn the initialization on a blocking task
        let ggwave = task::spawn_blocking(|| {
            GGWave::new()
        }).await.map_err(|_| Error::InitializationFailed)??;

        Ok(Self {
            inner: Arc::new(Mutex::new(ggwave)),
        })
    }

    /// Create a new AsyncGGWave instance with custom parameters using builder pattern
    ///
    /// # Examples
    ///
    /// ```
    /// use ggwave_rs::async_impl::AsyncGGWave;
    /// use ggwave_rs::sample_formats;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ggwave = AsyncGGWave::builder()
    ///         .sample_rate(48000.0)
    ///         .output_sample_format(sample_formats::F32)
    ///         .build()
    ///         .await
    ///         .expect("Failed to initialize AsyncGGWave");
    /// }
    /// ```
    pub fn builder() -> AsyncGGWaveBuilder {
        AsyncGGWaveBuilder::new()
    }

    /// Create a new AsyncGGWave instance with fixed-length encoding
    ///
    /// # Arguments
    ///
    /// * `payload_length` - Fixed payload length to use (must be <= 64)
    /// * `operating_mode` - Operating mode to use
    pub async fn new_with_fixed_payload(payload_length: i32, operating_mode: i32) -> Result<Self> {
        let ggwave = task::spawn_blocking(move || {
            GGWave::new_with_fixed_payload(payload_length, operating_mode)
        }).await.map_err(|_| Error::InitializationFailed)??;

        Ok(Self {
            inner: Arc::new(Mutex::new(ggwave)),
        })
    }

    /// Create a new AsyncGGWave instance with custom parameters
    pub async fn new_with_params(params: Parameters) -> Result<Self> {
        let ggwave = task::spawn_blocking(move || {
            GGWave::new_with_params(params)
        }).await.map_err(|_| Error::InitializationFailed)??;

        Ok(Self {
            inner: Arc::new(Mutex::new(ggwave)),
        })
    }

    /// Calculate the required buffer size for encoding text
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    pub async fn calculate_encode_buffer_size(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
    ) -> Result<usize> {
        let text = text.to_string();
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.calculate_encode_buffer_size(&text, protocol_id, volume)
        }).await.map_err(|_| Error::EncodeFailed(-1))?
    }

    /// Encode text into audio data asynchronously
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
    /// use ggwave_rs::async_impl::AsyncGGWave;
    /// use ggwave_rs::protocols;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ggwave = AsyncGGWave::new().await.expect("Failed to initialize AsyncGGWave");
    ///     let waveform = ggwave.encode("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
    ///         .await
    ///         .expect("Failed to encode text");
    /// }
    /// ```
    pub async fn encode(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
    ) -> Result<Vec<u8>> {
        let text = text.to_string();
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.encode(&text, protocol_id, volume)
        }).await.map_err(|_| Error::EncodeFailed(-1))?
    }

    /// Encode text into a provided buffer asynchronously
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
    pub async fn encode_into_buffer(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
        buffer: &mut [u8],
    ) -> Result<usize> {
        // Since we need to modify the provided buffer, we can't easily move this
        // to a separate thread. We'll get a mutable reference to buffer which cannot
        // be moved across threads. Use a two-step approach:
        
        // 1. Calculate size and check buffer
        let size = self.calculate_encode_buffer_size(text, protocol_id, volume).await?;
        
        if buffer.len() < size {
            return Err(Error::BufferTooSmall {
                required: size,
                provided: buffer.len(),
            });
        }
        
        // 2. Perform the encoding in a blocking task with a copy of the text
        let text = text.to_string();
        let inner = self.inner.clone();
        
        // Create a temporary buffer for the encoded data
        let encoded = task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.encode(&text, protocol_id, volume)
        }).await.map_err(|_| Error::EncodeFailed(-1))??;
        
        // Copy the results to the provided buffer
        let len = encoded.len().min(buffer.len());
        buffer[..len].copy_from_slice(&encoded[..len]);
        
        Ok(len)
    }

    /// Decode raw audio data to text asynchronously
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
    /// use ggwave_rs::async_impl::AsyncGGWave;
    /// use ggwave_rs::protocols;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ggwave = AsyncGGWave::new().await.expect("Failed to initialize AsyncGGWave");
    ///     let waveform = ggwave.encode("Hello, World!", protocols::AUDIBLE_NORMAL, 50)
    ///         .await
    ///         .expect("Failed to encode text");
    ///
    ///     let decoded = ggwave.decode_to_string(&waveform, 1024)
    ///         .await
    ///         .expect("Failed to decode waveform");
    ///
    ///     assert_eq!(decoded, "Hello, World!");
    /// }
    /// ```
    pub async fn decode_to_string(&self, waveform: &[u8], max_payload_size: usize) -> Result<String> {
        let waveform = waveform.to_vec();
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.decode_to_string(&waveform, max_payload_size)
        }).await.map_err(|_| Error::DecodeFailed(-1))?
    }

    /// Process an audio chunk asynchronously
    ///
    /// This method is useful for real-time streaming audio processing.
    ///
    /// # Arguments
    ///
    /// * `audio_chunk` - The audio chunk to process
    /// * `max_payload_size` - The maximum size of the decoded payload
    ///
    /// # Returns
    ///
    /// A `Result` containing an Option with the decoded string if something was found
    pub async fn process_audio_chunk(
        &self,
        audio_chunk: &[u8],
        max_payload_size: usize,
    ) -> Result<Option<String>> {
        let audio_chunk = audio_chunk.to_vec();
        let inner = self.inner.clone();
        
        let result = task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            let mut buffer = vec![0u8; max_payload_size];
            match ggwave.process_audio_chunk(&audio_chunk, &mut buffer)? {
                Some(s) => Ok::<Option<String>, Error>(Some(s.to_string())),
                None => Ok::<Option<String>, Error>(None),
            }
        }).await.map_err(|_| Error::DecodeFailed(-1))??;
        
        Ok(result)
    }

    /// Encode text and save directly to a WAV file asynchronously
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
    /// use ggwave_rs::async_impl::AsyncGGWave;
    /// use ggwave_rs::protocols;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let ggwave = AsyncGGWave::new().await.expect("Failed to initialize AsyncGGWave");
    ///     ggwave.encode_to_wav_file("Hello, World!", protocols::AUDIBLE_NORMAL, 50, "hello.wav")
    ///         .await
    ///         .expect("Failed to encode and save WAV file");
    /// }
    /// ```
    pub async fn encode_to_wav_file<P: AsRef<Path>>(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
        path: P,
    ) -> Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        let text = text.to_string();
        let inner = self.inner.clone();
        
        // First, encode and convert to WAV in memory
        let wav_data = task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.encode_to_wav(&text, protocol_id, volume)
        }).await.map_err(|_| Error::EncodeFailed(-1))??;
        
        // Then write to file using tokio's async file IO
        fs::write(path_buf, wav_data).await.map_err(Error::IoError)
    }

    /// Encode text to WAV format in memory asynchronously
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
    pub async fn encode_to_wav(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
    ) -> Result<Vec<u8>> {
        let text = text.to_string();
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.encode_to_wav(&text, protocol_id, volume)
        }).await.map_err(|_| Error::EncodeFailed(-1))?
    }

    /// Stream encoded audio data to an async writer
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    /// * `writer` - The async writer to stream to
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    pub async fn stream_encoded<W: AsyncWrite + Unpin>(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
        writer: &mut W,
    ) -> Result<()> {
        // Encode in a blocking task
        let encoded = self.encode(text, protocol_id, volume).await?;
        
        // Write to the async writer
        writer.write_all(&encoded).await.map_err(Error::IoError)
    }

    /// Stream WAV-encoded audio data to an async writer
    ///
    /// # Arguments
    ///
    /// * `text` - The text to encode
    /// * `protocol_id` - The protocol to use for encoding
    /// * `volume` - The volume of the encoded audio (0-100)
    /// * `writer` - The async writer to stream to
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    pub async fn stream_wav<W: AsyncWrite + Unpin>(
        &self,
        text: &str,
        protocol_id: ProtocolId,
        volume: i32,
        writer: &mut W,
    ) -> Result<()> {
        // Encode to WAV in a blocking task
        let wav_data = self.encode_to_wav(text, protocol_id, volume).await?;
        
        // Write to the async writer
        writer.write_all(&wav_data).await.map_err(Error::IoError)
    }

    /// Process an audio stream for decoding
    ///
    /// # Arguments
    ///
    /// * `reader` - The async reader to stream from
    /// * `chunk_size` - The size of chunks to read at once
    /// * `max_payload_size` - The maximum size of the decoded payload
    /// * `callback` - Function to call when data is decoded
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure
    pub async fn process_audio_stream<R, F>(
        &self,
        reader: &mut R,
        chunk_size: usize,
        max_payload_size: usize,
        mut callback: F,
    ) -> Result<()>
    where
        R: AsyncRead + Unpin,
        F: FnMut(String) -> Result<()>,
    {
        let mut buffer = vec![0u8; chunk_size];
        
        loop {
            // Read a chunk from the stream
            let n = reader.read(&mut buffer).await.map_err(Error::IoError)?;
            if n == 0 {
                break; // End of stream
            }
            
            // Process the chunk
            if let Some(decoded) = self.process_audio_chunk(&buffer[..n], max_payload_size).await? {
                callback(decoded)?;
            }
        }
        
        Ok(())
    }

    /// Toggle reception of a specific protocol
    pub async fn toggle_rx_protocol(&self, protocol_id: ProtocolId, enabled: bool) {
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.toggle_rx_protocol(protocol_id, enabled);
        }).await.ok();
    }

    /// Toggle transmission of a specific protocol
    pub async fn toggle_tx_protocol(&self, protocol_id: ProtocolId, enabled: bool) {
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.toggle_tx_protocol(protocol_id, enabled);
        }).await.ok();
    }

    /// Enable all reception protocols
    pub async fn enable_all_rx_protocols(&self) {
        let inner = self.inner.clone();
        
        task::spawn_blocking(move || {
            let ggwave = inner.blocking_lock();
            ggwave.enable_all_rx_protocols();
        }).await.ok();
    }

    /// Create a clone of this AsyncGGWave instance
    ///
    /// This is useful for sharing the same underlying GGWave instance
    /// across multiple tasks.
    pub fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Builder for AsyncGGWave parameters
pub struct AsyncGGWaveBuilder {
    /// Inner builder for synchronous GGWave
    inner_builder: crate::GGWaveBuilder,
}

impl AsyncGGWaveBuilder {
    /// Create a new builder with default parameters
    pub fn new() -> Self {
        Self {
            inner_builder: crate::GGWave::builder(),
        }
    }

    /// Set the sample rate for input, output, and processing
    pub fn sample_rate(mut self, rate: f32) -> Self {
        self.inner_builder = self.inner_builder.sample_rate(rate);
        self
    }

    /// Set the input sample rate
    pub fn input_sample_rate(mut self, rate: f32) -> Self {
        self.inner_builder = self.inner_builder.input_sample_rate(rate);
        self
    }

    /// Set the output sample rate
    pub fn output_sample_rate(mut self, rate: f32) -> Self {
        self.inner_builder = self.inner_builder.output_sample_rate(rate);
        self
    }

    /// Set samples per frame
    pub fn samples_per_frame(mut self, samples: i32) -> Self {
        self.inner_builder = self.inner_builder.samples_per_frame(samples);
        self
    }

    /// Set input sample format
    pub fn input_sample_format(mut self, format: crate::SampleFormat) -> Self {
        self.inner_builder = self.inner_builder.input_sample_format(format);
        self
    }

    /// Set output sample format
    pub fn output_sample_format(mut self, format: crate::SampleFormat) -> Self {
        self.inner_builder = self.inner_builder.output_sample_format(format);
        self
    }

    /// Set sound marker threshold
    pub fn sound_marker_threshold(mut self, threshold: f32) -> Self {
        self.inner_builder = self.inner_builder.sound_marker_threshold(threshold);
        self
    }

    /// Set operating mode
    pub fn operating_mode(mut self, mode: i32) -> Self {
        self.inner_builder = self.inner_builder.operating_mode(mode);
        self
    }

    /// Set fixed payload length
    pub fn fixed_payload_length(mut self, length: i32) -> Self {
        self.inner_builder = self.inner_builder.fixed_payload_length(length);
        self
    }

    /// Build an AsyncGGWave instance with the configured parameters
    pub async fn build(self) -> Result<AsyncGGWave> {
        let inner_builder = self.inner_builder;
        
        let ggwave = task::spawn_blocking(move || {
            inner_builder.build()
        }).await.map_err(|_| Error::InitializationFailed)??;
        
        Ok(AsyncGGWave {
            inner: Arc::new(Mutex::new(ggwave)),
        })
    }
}

impl Default for AsyncGGWaveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream processing utilities for async audio handling
pub mod streams {
    use super::*;
    use tokio::sync::mpsc;
    use std::time::Duration;

    /// A receiver for decoded messages from an audio stream
    pub struct MessageReceiver {
        rx: mpsc::Receiver<String>,
    }

    impl MessageReceiver {
        /// Receive the next decoded message
        ///
        /// # Returns
        ///
        /// An Option containing the next message, or None if the channel is closed
        pub async fn recv(&mut self) -> Option<String> {
            self.rx.recv().await
        }

        /// Try to receive a message without blocking
        ///
        /// # Returns
        ///
        /// An Option containing a message if one is available, or None otherwise
        pub fn try_recv(&mut self) -> Option<String> {
            self.rx.try_recv().ok()
        }

        /// Receive a message with a timeout
        ///
        /// # Arguments
        ///
        /// * `timeout` - The maximum time to wait
        ///
        /// # Returns
        ///
        /// An Option containing a message if one is received before the timeout, or None otherwise
        pub async fn recv_timeout(&mut self, timeout: Duration) -> Option<String> {
            tokio::time::timeout(timeout, self.rx.recv()).await.ok().flatten()
        }
    }

    /// Start processing an audio stream in the background
    ///
    /// # Arguments
    ///
    /// * `ggwave` - The AsyncGGWave instance to use
    /// * `reader` - The async reader to stream from
    /// * `chunk_size` - The size of chunks to read at once
    /// * `max_payload_size` - The maximum size of the decoded payload
    /// * `buffer_size` - The size of the message channel buffer
    ///
    /// # Returns
    ///
    /// A `Result` containing a MessageReceiver that can be used to receive decoded messages
    pub async fn start_background_processing<R>(
        ggwave: AsyncGGWave,
        mut reader: R,
        chunk_size: usize,
        max_payload_size: usize,
        buffer_size: usize,
    ) -> Result<MessageReceiver>
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(buffer_size);
        
        // Spawn a task to process the audio stream
        tokio::spawn(async move {
            let mut buffer = vec![0u8; chunk_size];
            
            loop {
                // Read a chunk from the stream
                let n = match reader.read(&mut buffer).await {
                    Ok(n) => n,
                    Err(_) => break, // Error reading from stream
                };
                
                if n == 0 {
                    break; // End of stream
                }
                
                // Process the chunk
                if let Ok(Some(decoded)) = ggwave.process_audio_chunk(&buffer[..n], max_payload_size).await {
                    // Try to send the decoded message
                    if tx.send(decoded).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            }
        });
        
        Ok(MessageReceiver { rx })
    }
}

#[cfg(test)]
mod tests {
    use crate::{protocols, sample_formats};

    use super::*;
    
    #[tokio::test]
    async fn test_async_encode_decode() {
        let ggwave = AsyncGGWave::new().await.expect("Failed to initialize AsyncGGWave");
        let text = "Hello, Async GGWave!";
        
        let waveform = ggwave.encode(text, protocols::AUDIBLE_NORMAL, 50)
            .await
            .expect("Failed to encode text");
            
        let decoded = ggwave.decode_to_string(&waveform, 1024)
            .await
            .expect("Failed to decode waveform");
            
        assert_eq!(decoded, text);
    }
    
    #[tokio::test]
    async fn test_async_builder() {
        let ggwave = AsyncGGWave::builder()
            .sample_rate(48000.0)
            .output_sample_format(sample_formats::F32)
            .build()
            .await
            .expect("Failed to initialize AsyncGGWave with builder");
            
        let text = "Testing builder";
        let waveform = ggwave.encode(text, protocols::AUDIBLE_NORMAL, 50)
            .await
            .expect("Failed to encode text");
            
        let decoded = ggwave.decode_to_string(&waveform, 1024)
            .await
            .expect("Failed to decode waveform");
            
        assert_eq!(decoded, text);
    }
}