// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Frontier.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use codec::Encode;
use lru::LruCache;

pub struct LRUCacheByteLimited<K, V> {
	cache: LruCache<K, V>,
	max_size: u64,
	metrics: Option<LRUCacheByteLimitedMetrics>,
	size: u64,
}

impl<K: Eq + Copy + core::hash::Hash, V: Encode> LRUCacheByteLimited<K, V> {
	pub fn new(
		cache_name: &'static str,
		max_size: u64,
		prometheus_registry: Option<prometheus_endpoint::Registry>,
	) -> Self {
		let metrics = match prometheus_registry {
			Some(registry) => match LRUCacheByteLimitedMetrics::register(cache_name, &registry) {
				Ok(metrics) => Some(metrics),
				Err(e) => {
					log::error!(target: "eth-cache", "Failed to register metrics: {:?}", e);
					None
				}
			},
			None => None,
		};

		Self {
			cache: LruCache::unbounded(),
			max_size,
			metrics,
			size: 0,
		}
	}
	pub fn get(&mut self, k: &K) -> Option<&V> {
		if let Some(ref v) = self.cache.get(k) {
			// Update metrics
			if let Some(ref metrics) = self.metrics {
				metrics.hits.inc();
			}
			Some(v)
		} else {
			// Update metrics
			if let Some(ref metrics) = self.metrics {
				metrics.miss.inc();
			}
			None
		}
	}
	pub fn put(&mut self, k: K, v: V) {
		// Handle size limit
		self.size += v.encoded_size() as u64;
		if self.size > self.max_size {
			let keys_to_remove = self
				.cache
				.iter()
				.rev()
				.take_while(|(_k, v)| {
					if self.size > self.max_size {
						let v_size = v.encoded_size() as u64;
						self.size -= v_size;
						true
					} else {
						false
					}
				})
				.map(|(k, _v)| *k)
				.collect::<Vec<_>>();
			for key in keys_to_remove {
				self.cache.pop(&key);
			}
		}
		// Add entry in cache
		self.cache.put(k, v);
		// Update metrics
		if let Some(ref metrics) = self.metrics {
			metrics
				.size
				.set(self.size.try_into().unwrap_or(std::u64::MAX));
		}
	}
}

struct LRUCacheByteLimitedMetrics {
	hits: prometheus::IntCounter,
	miss: prometheus::IntCounter,
	size: prometheus_endpoint::Gauge<prometheus_endpoint::U64>,
}

impl LRUCacheByteLimitedMetrics {
	pub(crate) fn register(
		cache_name: &'static str,
		registry: &prometheus_endpoint::Registry,
	) -> std::result::Result<Self, prometheus_endpoint::PrometheusError> {
		Ok(Self {
			hits: prometheus_endpoint::register(
				prometheus::IntCounter::new(
					format!("frontier_eth_{}_hits", cache_name),
					format!("Hits of eth {} cache.", cache_name),
				)?,
				registry,
			)?,
			miss: prometheus_endpoint::register(
				prometheus::IntCounter::new(
					format!("frontier_eth_{}_miss", cache_name),
					format!("Misses of eth {} cache.", cache_name),
				)?,
				registry,
			)?,
			size: prometheus_endpoint::register(
				prometheus_endpoint::Gauge::new(
					format!("frontier_eth_{}_size", cache_name),
					format!("Size of eth {} data cache.", cache_name),
				)?,
				registry,
			)?,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_size_limit() {
		let mut cache = LRUCacheByteLimited::new("name", 10, None);
		cache.put(0, "abcd");
		assert!(cache.get(&0).is_some());
		cache.put(1, "efghij");
		assert!(cache.get(&1).is_some());
		cache.put(2, "k");
		assert!(cache.get(&2).is_some());
		// Entry (0,  "abcd") should be deleted
		assert!(cache.get(&0).is_none());
		// Size should be 7 now, so we should be able to add a value of size 3
		cache.put(3, "lmn");
		assert!(cache.get(&3).is_some());
	}
}
