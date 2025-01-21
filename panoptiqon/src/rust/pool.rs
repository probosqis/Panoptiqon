/*
 * Copyright 2024-2025 wcaokaze
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
use std::sync::{Arc, RwLock};
use fnv::FnvHashMap;
use crate::cache::CacheContent;
use crate::unique_cache::UniqueCache;

#[cfg(feature="jvm")]
use {
   jni::JavaVM,
   jni::JNIEnv,
   crate::convert_jni::{CloneIntoJni, CloneIntoJavaHelper},
   crate::unique_cache::JvmUniqueCacheRefs,
};

#[cfg(feature="jvm")]
pub(crate) struct UniqueCachePool<T: CacheContent> {
   jvm: JavaVM,
   jvm_unique_cache_refs: Arc<JvmUniqueCacheRefs>,
   map: FnvHashMap<T::Key, Arc<RwLock<UniqueCache<T>>>>
}

#[cfg(not(feature="jvm"))]
pub(crate) struct UniqueCachePool<T: CacheContent> {
   map: FnvHashMap<T::Key, Arc<RwLock<UniqueCache<T>>>>
}

impl<T: CacheContent> UniqueCachePool<T> {
   pub fn get(&self, key: T::Key) -> Option<Arc<RwLock<UniqueCache<T>>>> {
      self.map.get(&key).map(|arc| arc.clone())
   }
}

#[cfg(feature="jvm")]
impl<T> UniqueCachePool<T>
   where T: CacheContent + CloneIntoJni
{
   pub fn new(env: &mut JNIEnv) -> Self
      where T: CloneIntoJavaHelper
   {
      let jvm = env.get_java_vm().unwrap();

      UniqueCachePool {
         jvm,
         jvm_unique_cache_refs: Arc::new(T::get_jvm_refs(env)),
         map: FnvHashMap::default()
      }
   }

   pub fn update(&mut self, key: T::Key, value: T) -> Arc<RwLock<UniqueCache<T>>>
      where T: CloneIntoJavaHelper
   {
      use std::collections::hash_map::Entry;

      let entry = self.map.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            let mut cache_lock = arc.write().unwrap();
            cache_lock.save(value);
            arc.clone()
         }
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_arc(
               &self.jvm, self.jvm_unique_cache_refs.clone(), value
            );
            entry.insert(arc.clone());
            arc
         }
      }
   }
}

#[cfg(not(feature="jvm"))]
impl<T: CacheContent> UniqueCachePool<T> {
   pub fn new() -> Self {
      UniqueCachePool {
         map: FnvHashMap::default()
      }
   }

   pub fn update(&mut self, key: T::Key, value: T) -> Arc<RwLock<UniqueCache<T>>> {
      use std::collections::hash_map::Entry;

      let entry = self.map.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            let mut cache_lock = arc.write().unwrap();
            cache_lock.save(value);
            arc.clone()
         }
         Entry::Vacant(entry) => {
            let unique_cache = UniqueCache::new(value);
            let arc = Arc::new(RwLock::new(unique_cache));
            entry.insert(arc.clone());
            arc
         }
      }
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::JObject;
   use crate::cache::CacheContent;
   use crate::convert_jni::CloneIntoJni;
   use super::UniqueCachePool;

   #[derive(Debug, PartialEq, Eq)]
   struct Content(String, i32);

   impl CacheContent for Content {
      type Key = String;

      fn key(&self) -> String {
         self.0.clone()
      }
   }

   impl CloneIntoJni for Content {
      fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
         self.0.clone_into_jni(env)
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_createCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);

      let cache = pool.update("A".to_string(), Content("A".to_string(), 42));
      let unique_cache = cache.read().unwrap();
      assert_eq!(Content("A".to_string(), 42), **unique_cache);

      let cache = pool.update("B".to_string(), Content("B".to_string(), 43));
      let unique_cache = cache.read().unwrap();
      assert_eq!(Content("B".to_string(), 43), **unique_cache);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_getCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);

      let _ = pool.update("A".to_string(), Content("A".to_string(), 42));

      let cache = pool.get("A".to_string());
      assert!(cache.is_some());
      let unique_cache = cache.unwrap();
      let cache_lock = unique_cache.read().unwrap();
      assert_eq!(Content("A".to_string(), 42), **cache_lock);

      let cache = pool.get("B".to_string());
      assert!(cache.is_none());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_pooling(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::sync::Arc;

      let mut pool = UniqueCachePool::new(&mut env);
      let cache1_ptr = Arc::as_ptr(&pool.update("A".to_string(), Content("A".to_string(), 42))) as *const _;
      let cache2_ptr = Arc::as_ptr(&pool.update("A".to_string(), Content("A".to_string(), 42))) as *const _;
      let cache3_ptr = Arc::as_ptr(&pool.update("B".to_string(), Content("B".to_string(), 42))) as *const _;

      assert_eq!(cache1_ptr, cache2_ptr);
      assert_ne!(cache1_ptr, cache3_ptr);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_save(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = UniqueCachePool::new(&mut env);
      let cache = pool.update("A".to_string(), Content("A".to_string(), 42));

      {
         let mut lock = cache.write().unwrap();
         assert_eq!(Content("A".to_string(), 42), **lock);
         lock.save(Content("A".to_string(), 43));
         assert_eq!(Content("A".to_string(), 43), **lock);
      }

      {
         let lock = cache.read().unwrap();
         assert_eq!(Content("A".to_string(), 43), **lock);
      }
   }
}
