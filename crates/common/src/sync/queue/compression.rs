use std::io::{Read, Write};

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use flate2::Compression;

use crate::error::CommonError;
use crate::sync::queue::errors::QueueResult;

/// Compression algorithms supported
#[derive(Debug, Clone, Copy)]
pub enum CompressionAlgorithm {
    Gzip,
    Zlib,
}

/// Compression service for queue data
pub struct CompressionService {
    algorithm: CompressionAlgorithm,
    level: u32,
}

impl CompressionService {
    /// Create new compression service
    pub fn new(algorithm: CompressionAlgorithm, level: u32) -> Self {
        Self { algorithm, level: level.min(9) }
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> QueueResult<Vec<u8>> {
        let compression_level = Compression::new(self.level);

        match self.algorithm {
            CompressionAlgorithm::Gzip => {
                let mut encoder = GzEncoder::new(Vec::new(), compression_level);
                encoder.write_all(data).map_err(|e| {
                    CommonError::internal(format!("Gzip compression failed: {}", e))
                })?;
                encoder.finish().map_err(|e| {
                    CommonError::internal(format!("Gzip finalization failed: {}", e)).into()
                })
            }
            CompressionAlgorithm::Zlib => {
                let mut encoder = ZlibEncoder::new(Vec::new(), compression_level);
                encoder.write_all(data).map_err(|e| {
                    CommonError::internal(format!("Zlib compression failed: {}", e))
                })?;
                encoder.finish().map_err(|e| {
                    CommonError::internal(format!("Zlib finalization failed: {}", e)).into()
                })
            }
        }
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> QueueResult<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::Gzip => {
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    CommonError::internal(format!("Gzip decompression failed: {}", e))
                })?;
                Ok(decompressed)
            }
            CompressionAlgorithm::Zlib => {
                let mut decoder = ZlibDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    CommonError::internal(format!("Zlib decompression failed: {}", e))
                })?;
                Ok(decompressed)
            }
        }
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self, original: usize, compressed: usize) -> f64 {
        if original == 0 {
            return 0.0;
        }
        (1.0 - (compressed as f64 / original as f64)) * 100.0
    }

    /// Estimate compressed size (heuristic)
    pub fn estimate_compressed_size(&self, original_size: usize) -> usize {
        // Rough estimates based on typical compression ratios
        let ratio = match self.algorithm {
            CompressionAlgorithm::Gzip => match self.level {
                0..=3 => 0.7,
                4..=6 => 0.5,
                7..=9 => 0.4,
                _ => 0.5,
            },
            CompressionAlgorithm::Zlib => match self.level {
                0..=3 => 0.65,
                4..=6 => 0.45,
                7..=9 => 0.35,
                _ => 0.45,
            },
        };
        (original_size as f64 * ratio) as usize
    }
}

impl Default for CompressionService {
    fn default() -> Self {
        Self::new(CompressionAlgorithm::Gzip, 6)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::queue::compression.
    use super::*;

    /// Validates `CompressionService::new` behavior for the gzip compress
    /// decompress scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    /// - Ensures `compressed.len() < original.len()` evaluates to true.
    #[test]
    fn test_gzip_compress_decompress() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        // Use larger, more repetitive data for better compression
        let original =
            b"Hello, World! This is a test message that should compress well. ".repeat(10);

        let compressed = service.compress(&original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
        assert!(compressed.len() < original.len());
    }

    /// Validates `CompressionService::new` behavior for the zlib compress
    /// decompress scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    /// - Ensures `compressed.len() < original.len()` evaluates to true.
    #[test]
    fn test_zlib_compress_decompress() {
        let service = CompressionService::new(CompressionAlgorithm::Zlib, 6);
        // Use larger, more repetitive data for better compression
        let original =
            b"Hello, World! This is a test message that should compress well. ".repeat(10);

        let compressed = service.compress(&original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
        assert!(compressed.len() < original.len());
    }

    /// Validates `CompressionService::new` behavior for the compression ratio
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `ratio > 50.0` evaluates to true.
    #[test]
    fn test_compression_ratio() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        // Use much more repetitive data for better compression ratio
        let original = b"a".repeat(1000);

        let compressed = service.compress(&original).unwrap();
        let ratio = service.compression_ratio(original.len(), compressed.len());

        // Should achieve significant compression on repeated data
        assert!(ratio > 50.0);
    }

    /// Validates `CompressionService::new` behavior for the compression levels
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `compressed_high.len() <= compressed_low.len()` evaluates to
    ///   true.
    /// - Confirms `decompressed_low` equals `original`.
    /// - Confirms `decompressed_high` equals `original`.
    #[test]
    fn test_compression_levels() {
        let original = b"Test data that will be compressed at different levels";

        let service_low = CompressionService::new(CompressionAlgorithm::Gzip, 1);
        let service_high = CompressionService::new(CompressionAlgorithm::Gzip, 9);

        let compressed_low = service_low.compress(original).unwrap();
        let compressed_high = service_high.compress(original).unwrap();

        // Higher compression should produce smaller output
        assert!(compressed_high.len() <= compressed_low.len());

        // Both should decompress correctly
        let decompressed_low = service_low.decompress(&compressed_low).unwrap();
        let decompressed_high = service_high.decompress(&compressed_high).unwrap();

        assert_eq!(decompressed_low, original);
        assert_eq!(decompressed_high, original);
    }

    /// Validates `CompressionService::new` behavior for the empty data
    /// compression scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    #[test]
    fn test_empty_data_compression() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let original = b"";

        let compressed = service.compress(original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    /// Validates `CompressionService::new` behavior for the small data
    /// compression scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    #[test]
    fn test_small_data_compression() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let original = b"Hi";

        let compressed = service.compress(original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
        // Small data might not compress well
    }

    /// Validates `CompressionService::new` behavior for the large data
    /// compression scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    /// - Ensures `compressed.len() < original.len() / 100` evaluates to true.
    #[test]
    fn test_large_data_compression() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let original = vec![42u8; 1024 * 1024]; // 1MB of repeated data

        let compressed = service.compress(&original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
        // Should compress extremely well
        assert!(compressed.len() < original.len() / 100);
    }

    /// Validates `CompressionService::new` behavior for the json like data
    /// compression scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `json_data`.
    #[test]
    fn test_json_like_data_compression() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let json_data = br#"{"id":"123","name":"test","data":{"field1":"value1","field2":"value2","field3":"value3"}}"#;

        let compressed = service.compress(json_data).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, json_data);
    }

    /// Validates `CompressionService::new` behavior for the compression ratio
    /// zero original scenario.
    ///
    /// Assertions:
    /// - Confirms `ratio` equals `0.0`.
    #[test]
    fn test_compression_ratio_zero_original() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let ratio = service.compression_ratio(0, 100);

        assert_eq!(ratio, 0.0);
    }

    /// Validates `CompressionService::new` behavior for the compression ratio
    /// no compression scenario.
    ///
    /// Assertions:
    /// - Confirms `ratio` equals `0.0`.
    #[test]
    fn test_compression_ratio_no_compression() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let ratio = service.compression_ratio(100, 100);

        assert_eq!(ratio, 0.0);
    }

    /// Validates `CompressionService::new` behavior for the estimate compressed
    /// size scenario.
    ///
    /// Assertions:
    /// - Ensures `(400..=600).contains(&estimated)` evaluates to true.
    #[test]
    fn test_estimate_compressed_size() {
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 6);

        let original_size = 1000;
        let estimated = service.estimate_compressed_size(original_size);

        // Should estimate around 50%
        assert!((400..=600).contains(&estimated));
    }

    /// Validates `CompressionService::new` behavior for the level clamping
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    #[test]
    fn test_level_clamping() {
        // Level above 9 should be clamped
        let service = CompressionService::new(CompressionAlgorithm::Gzip, 15);
        let original = b"Test data";

        let compressed = service.compress(original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }

    /// Validates `CompressionService::new` behavior for the gzip vs zlib
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `gzip_decompressed` equals `original`.
    /// - Confirms `zlib_decompressed` equals `original`.
    #[test]
    fn test_gzip_vs_zlib() {
        let original = b"Test data for comparing compression algorithms";

        let gzip_service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let zlib_service = CompressionService::new(CompressionAlgorithm::Zlib, 6);

        let gzip_compressed = gzip_service.compress(original).unwrap();
        let zlib_compressed = zlib_service.compress(original).unwrap();

        // Both should decompress correctly with their respective decompressors
        let gzip_decompressed = gzip_service.decompress(&gzip_compressed).unwrap();
        let zlib_decompressed = zlib_service.decompress(&zlib_compressed).unwrap();

        assert_eq!(gzip_decompressed, original);
        assert_eq!(zlib_decompressed, original);
    }

    /// Validates `CompressionService::new` behavior for the cross algorithm
    /// decompression fails scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_cross_algorithm_decompression_fails() {
        let original = b"Test data";

        let gzip_service = CompressionService::new(CompressionAlgorithm::Gzip, 6);
        let zlib_service = CompressionService::new(CompressionAlgorithm::Zlib, 6);

        let gzip_compressed = gzip_service.compress(original).unwrap();

        // Trying to decompress gzip data with zlib should fail
        let result = zlib_service.decompress(&gzip_compressed);
        assert!(result.is_err());
    }

    /// Validates `CompressionService::default` behavior for the default service
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `decompressed` equals `original`.
    #[test]
    fn test_default_service() {
        let service = CompressionService::default();
        let original = b"Test data";

        let compressed = service.compress(original).unwrap();
        let decompressed = service.decompress(&compressed).unwrap();

        assert_eq!(decompressed, original);
    }
}
