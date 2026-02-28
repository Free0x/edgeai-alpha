//! Data Quality Scoring System for PoIE 2.0
//!
//! This module implements advanced data quality evaluation algorithms
//! for assessing the value of contributed data in the EdgeAI network.
//!
//! NOTE: This module is prepared for future integration with the main
//! consensus mechanism. Currently used for reference and testing.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Data Quality Scoring System for PoIE 2.0
///
/// This module implements advanced data quality evaluation algorithms
/// that consider multiple factors:
/// 1. Information Entropy - How much unique information is in the data
/// 2. Freshness - How recent is the data
/// 3. Completeness - Are all expected fields present
/// 4. Consistency - Does the data follow expected patterns
/// 5. Uniqueness - Is this data duplicated elsewhere

/// Quality score breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    /// Overall quality score (0.0 - 1.0)
    pub overall: f64,
    /// Information entropy score (0.0 - 1.0)
    pub entropy: f64,
    /// Data freshness score (0.0 - 1.0)
    pub freshness: f64,
    /// Data completeness score (0.0 - 1.0)
    pub completeness: f64,
    /// Data consistency score (0.0 - 1.0)
    pub consistency: f64,
    /// Data uniqueness score (0.0 - 1.0)
    pub uniqueness: f64,
    /// Detailed breakdown
    pub details: QualityDetails,
}

/// Detailed quality analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDetails {
    /// Raw entropy value (bits)
    pub raw_entropy: f64,
    /// Data size in bytes
    pub data_size: usize,
    /// Number of unique values
    pub unique_values: usize,
    /// Detected data type
    pub data_type: DataType,
    /// Anomaly flags
    pub anomalies: Vec<String>,
}

/// Detected data type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    /// Structured JSON data
    Json,
    /// Numeric sensor readings
    Numeric,
    /// Text/string data
    Text,
    /// Binary data
    Binary,
    /// Image data
    Image,
    /// Time series data
    TimeSeries,
    /// Unknown format
    Unknown,
}

/// Data Quality Analyzer
pub struct DataQualityAnalyzer {
    /// Minimum entropy threshold for valid data
    pub min_entropy: f64,
    /// Maximum allowed duplicate ratio
    pub max_duplicate_ratio: f64,
    /// Freshness decay factor (per hour)
    pub freshness_decay: f64,
    /// Weights for different quality factors
    pub weights: QualityWeights,
}

/// Weights for quality factors
#[derive(Debug, Clone)]
pub struct QualityWeights {
    pub entropy: f64,
    pub freshness: f64,
    pub completeness: f64,
    pub consistency: f64,
    pub uniqueness: f64,
}

impl Default for QualityWeights {
    fn default() -> Self {
        QualityWeights {
            entropy: 0.30,      // 30% weight on information content
            freshness: 0.15,    // 15% weight on data freshness
            completeness: 0.20, // 20% weight on completeness
            consistency: 0.15,  // 15% weight on consistency
            uniqueness: 0.20,   // 20% weight on uniqueness
        }
    }
}

impl DataQualityAnalyzer {
    pub fn new() -> Self {
        DataQualityAnalyzer {
            min_entropy: 2.0,
            max_duplicate_ratio: 0.3,
            freshness_decay: 0.01,
            weights: QualityWeights::default(),
        }
    }

    /// Analyze data quality and return a comprehensive score
    pub fn analyze(&self, data: &[u8], timestamp_age_hours: f64) -> QualityScore {
        // Calculate individual scores
        let (entropy_score, raw_entropy) = self.calculate_entropy_score(data);
        let freshness_score = self.calculate_freshness_score(timestamp_age_hours);
        let (completeness_score, data_type) = self.calculate_completeness_score(data);
        let consistency_score = self.calculate_consistency_score(data, &data_type);
        let (uniqueness_score, unique_values) = self.calculate_uniqueness_score(data);

        // Detect anomalies
        let anomalies = self.detect_anomalies(data, raw_entropy, &data_type);

        // Calculate weighted overall score
        let overall = entropy_score * self.weights.entropy
            + freshness_score * self.weights.freshness
            + completeness_score * self.weights.completeness
            + consistency_score * self.weights.consistency
            + uniqueness_score * self.weights.uniqueness;

        QualityScore {
            overall,
            entropy: entropy_score,
            freshness: freshness_score,
            completeness: completeness_score,
            consistency: consistency_score,
            uniqueness: uniqueness_score,
            details: QualityDetails {
                raw_entropy,
                data_size: data.len(),
                unique_values,
                data_type,
                anomalies,
            },
        }
    }

    /// Calculate entropy-based quality score
    fn calculate_entropy_score(&self, data: &[u8]) -> (f64, f64) {
        if data.is_empty() {
            return (0.0, 0.0);
        }

        // Calculate Shannon entropy
        let mut frequency = [0u64; 256];
        for &byte in data {
            frequency[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let p = count as f64 / len;
                entropy -= p * p.log2();
            }
        }

        // Normalize to 0-1 scale (max entropy is 8 bits for bytes)
        let normalized = entropy / 8.0;

        // Apply threshold - penalize very low entropy data
        let score = if entropy < self.min_entropy {
            normalized * 0.5 // Penalty for low entropy
        } else {
            normalized
        };

        (score.clamp(0.0, 1.0), entropy)
    }

    /// Calculate freshness score based on data age
    fn calculate_freshness_score(&self, age_hours: f64) -> f64 {
        // Exponential decay based on age
        let decay = (-self.freshness_decay * age_hours).exp();
        decay.clamp(0.0, 1.0)
    }

    /// Calculate completeness score and detect data type
    fn calculate_completeness_score(&self, data: &[u8]) -> (f64, DataType) {
        // Try to detect data type
        let data_type = self.detect_data_type(data);

        let score = match data_type {
            DataType::Json => self.score_json_completeness(data),
            DataType::Numeric => self.score_numeric_completeness(data),
            DataType::TimeSeries => self.score_timeseries_completeness(data),
            _ => 0.7, // Default score for unknown types
        };

        (score, data_type)
    }

    /// Detect the type of data
    fn detect_data_type(&self, data: &[u8]) -> DataType {
        if data.is_empty() {
            return DataType::Unknown;
        }

        // Check for JSON
        if let Ok(text) = std::str::from_utf8(data) {
            let trimmed = text.trim();
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                return DataType::Json;
            }

            // Check for numeric data (comma or newline separated numbers)
            if trimmed.chars().all(|c| {
                c.is_numeric() || c == '.' || c == ',' || c == '\n' || c == '-' || c.is_whitespace()
            }) {
                return DataType::Numeric;
            }

            // Check for time series pattern
            if trimmed.contains("timestamp") || trimmed.contains("time") {
                return DataType::TimeSeries;
            }

            return DataType::Text;
        }

        // Check for image signatures
        if data.len() > 8 {
            // PNG signature
            if data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                return DataType::Image;
            }
            // JPEG signature
            if data[0..2] == [0xFF, 0xD8] {
                return DataType::Image;
            }
        }

        DataType::Binary
    }

    /// Score JSON data completeness
    fn score_json_completeness(&self, data: &[u8]) -> f64 {
        if let Ok(text) = std::str::from_utf8(data) {
            // Count key-value pairs
            let key_count = text.matches(':').count();
            let brace_count = text.matches('{').count();

            // More keys = more complete data
            let key_score = (key_count as f64 / 10.0).min(1.0);

            // Nested structures indicate richer data
            let structure_score = (brace_count as f64 / 5.0).min(1.0);

            return (key_score * 0.7 + structure_score * 0.3).clamp(0.0, 1.0);
        }
        0.5
    }

    /// Score numeric data completeness
    fn score_numeric_completeness(&self, data: &[u8]) -> f64 {
        if let Ok(text) = std::str::from_utf8(data) {
            let values: Vec<&str> = text
                .split(|c| c == ',' || c == '\n')
                .filter(|s| !s.trim().is_empty())
                .collect();

            // More data points = more complete
            let count_score = (values.len() as f64 / 100.0).min(1.0);

            // Check for valid numbers
            let valid_count = values
                .iter()
                .filter(|v| v.trim().parse::<f64>().is_ok())
                .count();
            let validity_score = valid_count as f64 / values.len().max(1) as f64;

            return (count_score * 0.5 + validity_score * 0.5).clamp(0.0, 1.0);
        }
        0.5
    }

    /// Score time series data completeness
    fn score_timeseries_completeness(&self, data: &[u8]) -> f64 {
        // For time series, we want regular intervals and no gaps
        self.score_json_completeness(data) * 0.9 // Slight penalty for complexity
    }

    /// Calculate consistency score
    fn calculate_consistency_score(&self, data: &[u8], data_type: &DataType) -> f64 {
        match data_type {
            DataType::Json => self.score_json_consistency(data),
            DataType::Numeric => self.score_numeric_consistency(data),
            _ => 0.8, // Default consistency score
        }
    }

    /// Score JSON consistency
    fn score_json_consistency(&self, data: &[u8]) -> f64 {
        if let Ok(text) = std::str::from_utf8(data) {
            // Check for balanced braces
            let open_braces = text.matches('{').count();
            let close_braces = text.matches('}').count();
            let open_brackets = text.matches('[').count();
            let close_brackets = text.matches(']').count();

            if open_braces != close_braces || open_brackets != close_brackets {
                return 0.3; // Malformed JSON
            }

            // Check for proper quoting
            let quote_count = text.matches('"').count();
            if quote_count % 2 != 0 {
                return 0.5; // Unbalanced quotes
            }

            return 0.95;
        }
        0.5
    }

    /// Score numeric consistency
    fn score_numeric_consistency(&self, data: &[u8]) -> f64 {
        if let Ok(text) = std::str::from_utf8(data) {
            let values: Vec<f64> = text
                .split(|c| c == ',' || c == '\n')
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .collect();

            if values.len() < 2 {
                return 0.5;
            }

            // Calculate coefficient of variation
            let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
            let variance: f64 =
                values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
            let std_dev = variance.sqrt();

            let cv = if mean.abs() > 0.001 {
                std_dev / mean.abs()
            } else {
                std_dev
            };

            // Lower CV = more consistent (but not too low, which might indicate fake data)
            if cv < 0.01 {
                return 0.5; // Suspiciously consistent
            } else if cv > 2.0 {
                return 0.6; // Very inconsistent
            } else {
                return 0.9;
            }
        }
        0.5
    }

    /// Calculate uniqueness score
    fn calculate_uniqueness_score(&self, data: &[u8]) -> (f64, usize) {
        if data.is_empty() {
            return (0.0, 0);
        }

        // Count unique byte patterns (4-byte windows)
        let mut unique_patterns = std::collections::HashSet::new();

        for window in data.windows(4) {
            let pattern = u32::from_be_bytes([window[0], window[1], window[2], window[3]]);
            unique_patterns.insert(pattern);
        }

        let unique_count = unique_patterns.len();
        let total_windows = data.len().saturating_sub(3);

        if total_windows == 0 {
            return (0.5, unique_count);
        }

        let uniqueness_ratio = unique_count as f64 / total_windows as f64;

        // Score based on uniqueness ratio
        let score = if uniqueness_ratio > self.max_duplicate_ratio {
            uniqueness_ratio.min(1.0)
        } else {
            uniqueness_ratio * 0.7 // Penalty for high duplication
        };

        (score.clamp(0.0, 1.0), unique_count)
    }

    /// Detect anomalies in the data
    fn detect_anomalies(&self, data: &[u8], entropy: f64, _data_type: &DataType) -> Vec<String> {
        let mut anomalies = Vec::new();

        // Check for suspiciously low entropy
        if entropy < 1.0 {
            anomalies.push("Very low entropy - possible fake/generated data".to_string());
        }

        // Check for suspiciously high entropy (random noise)
        if entropy > 7.9 {
            anomalies.push("Very high entropy - possible random noise".to_string());
        }

        // Check for small data size
        if data.len() < 10 {
            anomalies.push("Data too small to be meaningful".to_string());
        }

        // Check for all zeros
        if data.iter().all(|&b| b == 0) {
            anomalies.push("Data is all zeros".to_string());
        }

        // Check for all same value
        if !data.is_empty() && data.iter().all(|&b| b == data[0]) {
            anomalies.push("Data is all same value".to_string());
        }

        anomalies
    }

    /// Calculate contribution points based on quality score
    pub fn calculate_points(&self, quality: &QualityScore, base_points: f64) -> f64 {
        // Base points multiplied by quality
        let quality_multiplier = quality.overall;

        // Bonus for high entropy data
        let entropy_bonus = if quality.entropy > 0.8 { 1.2 } else { 1.0 };

        // Penalty for anomalies
        let anomaly_penalty = if quality.details.anomalies.is_empty() {
            1.0
        } else {
            0.8_f64.powi(quality.details.anomalies.len() as i32)
        };

        base_points * quality_multiplier * entropy_bonus * anomaly_penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_calculation() {
        let analyzer = DataQualityAnalyzer::new();

        // Random-like data should have high entropy
        let random_data: Vec<u8> = (0..256).collect();
        let (score, entropy) = analyzer.calculate_entropy_score(&random_data);
        assert!(entropy > 7.0);
        assert!(score > 0.8);

        // Repetitive data should have low entropy
        let repetitive_data = vec![0u8; 256];
        let (score, entropy) = analyzer.calculate_entropy_score(&repetitive_data);
        assert!(entropy < 1.0);
        assert!(score < 0.2);
    }

    #[test]
    fn test_json_detection() {
        let analyzer = DataQualityAnalyzer::new();

        let json_data = b"{\"temperature\": 25.5, \"humidity\": 60}";
        let data_type = analyzer.detect_data_type(json_data);
        assert_eq!(data_type, DataType::Json);
    }

    #[test]
    fn test_quality_analysis() {
        let analyzer = DataQualityAnalyzer::new();

        let good_data = b"{\"device_id\": \"sensor_001\", \"temperature\": 25.5, \"humidity\": 60, \"timestamp\": 1704931200}";
        let quality = analyzer.analyze(good_data, 0.5);

        assert!(quality.overall > 0.5);
        assert!(quality.details.anomalies.is_empty());
    }
}
