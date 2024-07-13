/*
 * Copyright 2024 wcaokaze
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::borrow::Borrow;
use std::hash::Hash;

use fnv::FnvHashMap;

use crate::cache::Cache;

pub(crate) struct CachePool<K, T> {
   map: FnvHashMap<K, Cache<T>>
}

impl<K, T> CachePool<K, T>
   where K: Hash + Eq
{
   pub fn new() -> Self {
      CachePool {
         map: FnvHashMap::default()
      }
   }

   pub fn get(&mut self, key: K, initial_value: impl Fn() -> T) -> &Cache<T> {
      self.map.entry(key).or_insert_with(|| {
         let initial_value = initial_value();
         Cache::new(initial_value)
      })
   }
}

#[cfg(test)]
mod test {
   use super::CachePool;

   #[test]
   fn create_cache() {
      let mut pool = CachePool::new();
      let cache = pool.get("A".to_string(), || 42);
      assert_eq!(42, **cache);

      let cache = pool.get("B".to_string(), || 43);
      assert_eq!(43, **cache);
   }

   #[test]
   fn pooling() {
      let mut pool = CachePool::new();
      let cache1_ptr = pool.get("A".to_string(), || 42) as *const _;
      let cache2_ptr = pool.get("A".to_string(), || 42) as *const _;
      let cache3_ptr = pool.get("B".to_string(), || 42) as *const _;

      assert_eq!(cache1_ptr, cache2_ptr);
      assert_ne!(cache1_ptr, cache3_ptr);
   }
}
