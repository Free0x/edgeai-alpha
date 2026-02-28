//! Data Marketplace module for EdgeAI Blockchain
//!
//! This module provides a decentralized marketplace for data trading,
//! including data listing, purchasing, and quality-based pricing.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Data category for marketplace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DataCategory {
    IoTSensor,
    AIModel,
    TrainingData,
    ImageData,
    AudioData,
    TextData,
    LocationData,
    HealthData,
    EnvironmentData,
    IndustrialData,
    Custom(String),
}

impl DataCategory {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "iot" | "iot_sensor" | "sensor" => DataCategory::IoTSensor,
            "ai" | "ai_model" | "model" => DataCategory::AIModel,
            "training" | "training_data" => DataCategory::TrainingData,
            "image" | "image_data" => DataCategory::ImageData,
            "audio" | "audio_data" => DataCategory::AudioData,
            "text" | "text_data" => DataCategory::TextData,
            "location" | "gps" => DataCategory::LocationData,
            "health" | "medical" => DataCategory::HealthData,
            "environment" | "weather" => DataCategory::EnvironmentData,
            "industrial" | "manufacturing" => DataCategory::IndustrialData,
            _ => DataCategory::Custom(s.to_string()),
        }
    }
}

/// Data listing in the marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataListing {
    pub id: String,
    pub data_hash: String,
    pub owner: String,
    pub title: String,
    pub description: String,
    pub category: DataCategory,
    pub price: u64,
    pub quality_score: f64,
    pub entropy_score: f64,
    pub size_bytes: u64,
    pub sample_data: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
    pub total_purchases: u64,
    pub total_revenue: u64,
    pub ratings: Vec<DataRating>,
    pub tags: Vec<String>,
}

impl DataListing {
    pub fn new(
        data_hash: String,
        owner: String,
        title: String,
        description: String,
        category: DataCategory,
        price: u64,
        quality_score: f64,
        entropy_score: f64,
        size_bytes: u64,
    ) -> Self {
        let id = format!("listing_{}", &data_hash[..16]);
        let now = Utc::now();

        DataListing {
            id,
            data_hash,
            owner,
            title,
            description,
            category,
            price,
            quality_score,
            entropy_score,
            size_bytes,
            sample_data: None,
            created_at: now,
            updated_at: now,
            is_active: true,
            total_purchases: 0,
            total_revenue: 0,
            ratings: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn average_rating(&self) -> f64 {
        if self.ratings.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.ratings.iter().map(|r| r.score).sum();
        sum / self.ratings.len() as f64
    }
}

/// Rating for a data listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRating {
    pub buyer: String,
    pub score: f64, // 1-5
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Purchase record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRecord {
    pub id: String,
    pub listing_id: String,
    pub data_hash: String,
    pub buyer: String,
    pub seller: String,
    pub price: u64,
    pub purchased_at: DateTime<Utc>,
    pub access_granted: bool,
}

/// Data marketplace
pub struct DataMarketplace {
    pub listings: HashMap<String, DataListing>,
    pub purchases: Vec<PurchaseRecord>,
    pub category_index: HashMap<DataCategory, Vec<String>>, // category -> listing_ids
    pub owner_index: HashMap<String, Vec<String>>,          // owner -> listing_ids
    pub platform_fee_rate: f64,                             // Platform fee percentage (0.0 - 1.0)
    pub total_volume: u64,
}

impl DataMarketplace {
    pub fn new() -> Self {
        DataMarketplace {
            listings: HashMap::new(),
            purchases: Vec::new(),
            category_index: HashMap::new(),
            owner_index: HashMap::new(),
            platform_fee_rate: 0.025, // 2.5% platform fee
            total_volume: 0,
        }
    }

    /// List new data for sale
    pub fn list_data(&mut self, listing: DataListing) -> Result<String, String> {
        // Validate listing
        if listing.price == 0 {
            return Err("Price must be greater than 0".to_string());
        }

        if listing.quality_score < 0.0 || listing.quality_score > 1.0 {
            return Err("Quality score must be between 0 and 1".to_string());
        }

        // Check for duplicate
        if self.listings.contains_key(&listing.data_hash) {
            return Err("Data already listed".to_string());
        }

        let listing_id = listing.id.clone();
        let category = listing.category.clone();
        let owner = listing.owner.clone();
        let data_hash = listing.data_hash.clone();

        // Add to main index
        self.listings.insert(data_hash.clone(), listing);

        // Add to category index
        self.category_index
            .entry(category)
            .or_insert_with(Vec::new)
            .push(data_hash.clone());

        // Add to owner index
        self.owner_index
            .entry(owner.clone())
            .or_insert_with(Vec::new)
            .push(data_hash.clone());

        info!("Data listed: {} by {}", &listing_id[..16], &owner[..8]);

        Ok(listing_id)
    }

    /// Purchase data
    pub fn purchase_data(
        &mut self,
        data_hash: &str,
        buyer: &str,
    ) -> Result<PurchaseRecord, String> {
        let listing = self
            .listings
            .get_mut(data_hash)
            .ok_or("Listing not found")?;

        if !listing.is_active {
            return Err("Listing is not active".to_string());
        }

        if listing.owner == buyer {
            return Err("Cannot purchase own data".to_string());
        }

        // Create purchase record
        let purchase = PurchaseRecord {
            id: format!("purchase_{}_{}", &data_hash[..8], self.purchases.len()),
            listing_id: listing.id.clone(),
            data_hash: data_hash.to_string(),
            buyer: buyer.to_string(),
            seller: listing.owner.clone(),
            price: listing.price,
            purchased_at: Utc::now(),
            access_granted: true,
        };

        // Update listing stats
        listing.total_purchases += 1;
        listing.total_revenue += listing.price;
        listing.updated_at = Utc::now();

        // Update marketplace stats
        self.total_volume += listing.price;

        self.purchases.push(purchase.clone());

        info!(
            "Data purchased: {} by {} for {} tokens",
            &data_hash[..8],
            &buyer[..8],
            listing.price
        );

        Ok(purchase)
    }

    /// Rate a purchased data
    pub fn rate_data(
        &mut self,
        data_hash: &str,
        buyer: &str,
        score: f64,
        comment: Option<String>,
    ) -> Result<(), String> {
        // Verify purchase
        let has_purchased = self
            .purchases
            .iter()
            .any(|p| p.data_hash == data_hash && p.buyer == buyer);

        if !has_purchased {
            return Err("Must purchase data before rating".to_string());
        }

        if score < 1.0 || score > 5.0 {
            return Err("Rating must be between 1 and 5".to_string());
        }

        let listing = self
            .listings
            .get_mut(data_hash)
            .ok_or("Listing not found")?;

        // Check if already rated
        if listing.ratings.iter().any(|r| r.buyer == buyer) {
            return Err("Already rated this data".to_string());
        }

        listing.ratings.push(DataRating {
            buyer: buyer.to_string(),
            score,
            comment,
            created_at: Utc::now(),
        });

        Ok(())
    }

    /// Update listing price
    pub fn update_price(
        &mut self,
        data_hash: &str,
        owner: &str,
        new_price: u64,
    ) -> Result<(), String> {
        let listing = self
            .listings
            .get_mut(data_hash)
            .ok_or("Listing not found")?;

        if listing.owner != owner {
            return Err("Not the owner".to_string());
        }

        listing.price = new_price;
        listing.updated_at = Utc::now();

        Ok(())
    }

    /// Deactivate listing
    pub fn deactivate_listing(&mut self, data_hash: &str, owner: &str) -> Result<(), String> {
        let listing = self
            .listings
            .get_mut(data_hash)
            .ok_or("Listing not found")?;

        if listing.owner != owner {
            return Err("Not the owner".to_string());
        }

        listing.is_active = false;
        listing.updated_at = Utc::now();

        Ok(())
    }

    /// Get listing by data hash
    pub fn get_listing(&self, data_hash: &str) -> Option<&DataListing> {
        self.listings.get(data_hash)
    }

    /// Get listings by category
    pub fn get_by_category(&self, category: &DataCategory) -> Vec<&DataListing> {
        self.category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.listings.get(id))
                    .filter(|l| l.is_active)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get listings by owner
    pub fn get_by_owner(&self, owner: &str) -> Vec<&DataListing> {
        self.owner_index
            .get(owner)
            .map(|ids| ids.iter().filter_map(|id| self.listings.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get purchases by buyer
    pub fn get_purchases_by_buyer(&self, buyer: &str) -> Vec<&PurchaseRecord> {
        self.purchases.iter().filter(|p| p.buyer == buyer).collect()
    }

    /// Search listings
    pub fn search(
        &self,
        query: Option<&str>,
        category: Option<&DataCategory>,
        min_price: Option<u64>,
        max_price: Option<u64>,
        min_quality: Option<f64>,
        sort_by: SortBy,
        limit: usize,
    ) -> Vec<&DataListing> {
        let mut results: Vec<&DataListing> = self
            .listings
            .values()
            .filter(|l| l.is_active)
            .filter(|l| {
                if let Some(q) = query {
                    let q_lower = q.to_lowercase();
                    l.title.to_lowercase().contains(&q_lower)
                        || l.description.to_lowercase().contains(&q_lower)
                        || l.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
                } else {
                    true
                }
            })
            .filter(|l| category.map(|c| &l.category == c).unwrap_or(true))
            .filter(|l| min_price.map(|p| l.price >= p).unwrap_or(true))
            .filter(|l| max_price.map(|p| l.price <= p).unwrap_or(true))
            .filter(|l| min_quality.map(|q| l.quality_score >= q).unwrap_or(true))
            .collect();

        // Sort results
        match sort_by {
            SortBy::PriceAsc => results.sort_by(|a, b| a.price.cmp(&b.price)),
            SortBy::PriceDesc => results.sort_by(|a, b| b.price.cmp(&a.price)),
            SortBy::QualityDesc => {
                results.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap())
            }
            SortBy::PopularityDesc => {
                results.sort_by(|a, b| b.total_purchases.cmp(&a.total_purchases))
            }
            SortBy::Newest => results.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            SortBy::RatingDesc => {
                results.sort_by(|a, b| b.average_rating().partial_cmp(&a.average_rating()).unwrap())
            }
        }

        results.truncate(limit);
        results
    }

    /// Get marketplace statistics
    pub fn get_stats(&self) -> MarketplaceStats {
        let active_listings = self.listings.values().filter(|l| l.is_active).count();

        let total_listings = self.listings.len();
        let total_purchases = self.purchases.len();

        let avg_price = if active_listings > 0 {
            self.listings
                .values()
                .filter(|l| l.is_active)
                .map(|l| l.price)
                .sum::<u64>()
                / active_listings as u64
        } else {
            0
        };

        let category_counts: HashMap<String, usize> = self
            .category_index
            .iter()
            .map(|(cat, ids)| (format!("{:?}", cat), ids.len()))
            .collect();

        MarketplaceStats {
            total_listings: total_listings as u64,
            active_listings: active_listings as u64,
            total_purchases: total_purchases as u64,
            total_volume: self.total_volume,
            average_price: avg_price,
            category_counts,
            unique_sellers: self.owner_index.len() as u64,
        }
    }
}

/// Sort options for search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortBy {
    PriceAsc,
    PriceDesc,
    QualityDesc,
    PopularityDesc,
    Newest,
    RatingDesc,
}

/// Marketplace statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceStats {
    pub total_listings: u64,
    pub active_listings: u64,
    pub total_purchases: u64,
    pub total_volume: u64,
    pub average_price: u64,
    pub category_counts: HashMap<String, usize>,
    pub unique_sellers: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_and_purchase() {
        let mut marketplace = DataMarketplace::new();

        let listing = DataListing::new(
            "hash123".to_string(),
            "seller1".to_string(),
            "Temperature Data".to_string(),
            "IoT sensor temperature readings".to_string(),
            DataCategory::IoTSensor,
            100,
            0.85,
            6.5,
            1024,
        );

        let result = marketplace.list_data(listing);
        assert!(result.is_ok());

        let purchase = marketplace.purchase_data("hash123", "buyer1");
        assert!(purchase.is_ok());

        let listing = marketplace.get_listing("hash123").unwrap();
        assert_eq!(listing.total_purchases, 1);
    }

    #[test]
    fn test_search() {
        let mut marketplace = DataMarketplace::new();

        for i in 0..5 {
            let listing = DataListing::new(
                format!("hash{}", i),
                "seller1".to_string(),
                format!("Data {}", i),
                "Test data".to_string(),
                DataCategory::IoTSensor,
                100 + i * 10,
                0.8 + i as f64 * 0.02,
                6.0,
                1024,
            );
            marketplace.list_data(listing).unwrap();
        }

        let results = marketplace.search(
            None,
            Some(&DataCategory::IoTSensor),
            None,
            None,
            None,
            SortBy::PriceDesc,
            10,
        );

        assert_eq!(results.len(), 5);
        assert!(results[0].price >= results[1].price);
    }
}
