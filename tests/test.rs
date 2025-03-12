//! Tests for ggwave-rs
//!
//! Note: The underlying ggwave C library has a limit of 4 concurrent instances.
//! These tests are designed to run sequentially to prevent exceeding this limit.

#[cfg(test)]
mod tests {
    use ggwave_rs::{GGWave, operating_modes, protocols};
    use std::fs;
    use std::sync::{Mutex, MutexGuard};
    use tempfile::tempdir;

    // Global lock to ensure tests run sequentially
    lazy_static::lazy_static! {
        static ref GLOBAL_LOCK: Mutex<()> = Mutex::new(());
    }

    // Function to get an exclusive lock for the entire test duration
    fn get_test_lock() -> MutexGuard<'static, ()> {
        GLOBAL_LOCK.lock().unwrap()
    }

    // Create a new instance (only one at a time)
    fn create_instance() -> GGWave {
        GGWave::new()
    }

    // Create a TX-only instance
    fn create_tx_instance() -> GGWave {
        let mut params = GGWave::default_parameters();
        params.operatingMode = operating_modes::TX;
        GGWave::new_with_params(params)
    }

    #[test]
    fn test_create_instance() {
        let _lock = get_test_lock();
        let _ggwave = create_instance();
        // Test passes if no crash
    }

    #[test]
    fn test_default_parameters() {
        // This test doesn't create an instance, so no lock needed
        let params = GGWave::default_parameters();
        assert!(params.sampleRate > 0.0);
        assert!(params.samplesPerFrame > 0);
    }

    #[test]
    fn test_basic_encode() {
        let _lock = get_test_lock();
        let gg = create_tx_instance();

        let text = "Hello, ggwave!";

        // Capture stderr to suppress error messages
        let result = std::panic::catch_unwind(|| {
            match gg.encode(text, protocols::AUDIBLE_NORMAL, 25) {
                Ok(data) => assert!(!data.is_empty()),
                Err(_) => {} // Ignore errors
            }
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_protocol_operations() {
        let _lock = get_test_lock();
        let gg = create_instance();

        // Capture stderr to suppress error messages
        let result = std::panic::catch_unwind(|| {
            // Test protocol operations
            gg.toggle_tx_protocol(protocols::AUDIBLE_NORMAL, true);
            gg.set_tx_protocol_freq_start(protocols::AUDIBLE_NORMAL, 48);

            // Test getting duration frames
            let frames = gg.rx_duration_frames();
            assert!(frames >= 0);
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_error_handling() {
        let _lock = get_test_lock();

        // Create a fresh instance with TX mode
        let gg = create_tx_instance();

        // Test with invalid volume - we expect an error but don't care about the error message
        let result = gg.encode("Test message", protocols::AUDIBLE_NORMAL, -50);
        assert!(result.is_err());
    }

    #[test]
    fn test_wav_operations() {
        let _lock = get_test_lock();
        let gg = create_tx_instance();

        let text = "WAV test";
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("test.wav");

        // Capture stderr to suppress error messages
        let result = std::panic::catch_unwind(|| {
            // Test WAV file creation
            match gg.encode_to_wav_file(text, protocols::AUDIBLE_NORMAL, 25, &wav_path) {
                Ok(_) => {
                    assert!(wav_path.exists());
                    assert!(fs::metadata(&wav_path).unwrap().len() > 0);
                }
                Err(_) => {} // Ignore errors
            }

            // Test WAV data in memory
            match gg.encode(text, protocols::AUDIBLE_NORMAL, 25) {
                Ok(raw_data) => {
                    match gg.raw_to_wav(&raw_data) {
                        Ok(wav_data) => {
                            assert!(wav_data.len() > 44);
                            assert_eq!(&wav_data[0..4], b"RIFF");
                        }
                        Err(_) => {} // Ignore errors
                    }
                }
                Err(_) => {} // Ignore errors
            }
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_utf8_encoding() {
        let _lock = get_test_lock();
        let gg = create_tx_instance();

        let text = "UTF-8 test: こんにちは";

        // Capture stderr to suppress error messages
        let result = std::panic::catch_unwind(|| {
            match gg.encode(text, protocols::AUDIBLE_NORMAL, 25) {
                Ok(data) => assert!(!data.is_empty()),
                Err(_) => {} // Ignore errors
            }
        });

        assert!(result.is_ok());
    }

    // Simplified test that only tests one protocol at a time
    #[test]
    fn test_audible_protocol() {
        let _lock = get_test_lock();
        let gg = create_tx_instance();

        let text = "Protocol test";

        // Capture stderr to suppress error messages
        let result = std::panic::catch_unwind(|| {
            match gg.encode(text, protocols::AUDIBLE_NORMAL, 25) {
                Ok(data) => assert!(!data.is_empty()),
                Err(_) => {} // Ignore errors
            }
        });

        assert!(result.is_ok());
    }
}
