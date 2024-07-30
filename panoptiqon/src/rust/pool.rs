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
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use fnv::FnvHashMap;

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   crate::unique_cache::JvmUniqueCacheRefs,
   jni::JavaVM,
   jni::JNIEnv,
};

use crate::unique_cache::UniqueCache;

#[cfg(feature="jvm")]
pub(crate) struct UniqueCachePool<K, T> {
   jvm: JavaVM,
   jvm_unique_cache_refs: JvmUniqueCacheRefs,
   map: FnvHashMap<K, Arc<Mutex<UniqueCache<T>>>>
}

#[cfg(not(feature="jvm"))]
pub(crate) struct UniqueCachePool<K, T> {
   map: FnvHashMap<K, Arc<Mutex<UniqueCache<T>>>>
}

impl<K, T> UniqueCachePool<K, T>
   where K: Hash + Eq
{
   pub fn get(&self, key: K) -> Option<Arc<Mutex<UniqueCache<T>>>> {
      self.map.get(&key).map(|arc| arc.clone())
   }
}

#[cfg(feature="jvm")]
impl<K, T> UniqueCachePool<K, T>
   where K: Hash + Eq,
         T: ConvertJava
{
   pub fn new(env: &mut JNIEnv) -> Self {
      let jvm = env.get_java_vm().unwrap();

      UniqueCachePool {
         jvm,
         jvm_unique_cache_refs: JvmUniqueCacheRefs::new(env),
         map: FnvHashMap::default()
      }
   }

   pub fn get_or_insert(
      &mut self,
      key: K,
      initial_value: impl Fn() -> T
   ) -> Arc<Mutex<UniqueCache<T>>> {
      let arc = self.map.entry(key).or_insert_with(|| {
         let initial_value = initial_value();
         UniqueCache::new_arc(&self.jvm, &self.jvm_unique_cache_refs, initial_value)
      });

      arc.clone()
   }
}

#[cfg(not(feature="jvm"))]
impl<K, T> UniqueCachePool<K, T>
   where K: Hash + Eq
{
   pub fn new() -> Self {
      UniqueCachePool {
         map: FnvHashMap::default()
      }
   }

   pub fn get_or_insert(
      &mut self,
      key: K,
      initial_value: impl Fn() -> T
   ) -> Arc<Mutex<UniqueCache<T>>> {
      let arc = self.map.entry(key).or_insert_with(|| {
         let initial_value = initial_value();
         let unique_cache = UniqueCache::new(initial_value);
         Arc::new(Mutex::new(unique_cache))
      });

      arc.clone()
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use std::sync::Arc;

   use jni::JNIEnv;
   use jni::objects::JObject;

   use super::UniqueCachePool;

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_createCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);

      let cache = pool.get_or_insert("A".to_string(), || 42);
      let unique_cache = cache.lock().unwrap();
      assert_eq!(42, **unique_cache);

      let cache = pool.get_or_insert("B".to_string(), || 43);
      let unique_cache = cache.lock().unwrap();
      assert_eq!(43, **unique_cache);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_getCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);

      let _ = pool.get_or_insert("A".to_string(), || 42);

      let cache = pool.get("A".to_string());
      assert!(cache.is_some());
      let unique_cache = cache.unwrap();
      let cache_lock = unique_cache.lock().unwrap();
      assert_eq!(42, **cache_lock);

      let cache = pool.get("B".to_string());
      assert!(cache.is_none());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_pooling(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);
      let cache1_ptr = Arc::as_ptr(&pool.get_or_insert("A".to_string(), || 42)) as *const _;
      let cache2_ptr = Arc::as_ptr(&pool.get_or_insert("A".to_string(), || 42)) as *const _;
      let cache3_ptr = Arc::as_ptr(&pool.get_or_insert("B".to_string(), || 42)) as *const _;

      assert_eq!(cache1_ptr, cache2_ptr);
      assert_ne!(cache1_ptr, cache3_ptr);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_save(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);
      let cache = pool.get_or_insert("A".to_string(), || 42);

      {
         let mut lock = cache.lock().unwrap();
         assert_eq!(42, **lock);
         lock.save(43);
         assert_eq!(43, **lock);
      }

      {
         let lock = cache.lock().unwrap();
         assert_eq!(43, **lock);
      }
   }
}
