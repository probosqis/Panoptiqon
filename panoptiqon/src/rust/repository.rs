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

use anyhow::{bail, Result};

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   jni::JNIEnv,
};

use crate::cache::Cache;
use crate::pool::UniqueCachePool;

pub struct Repository<K, T> {
   pool: UniqueCachePool<K, T>
}

impl<K, T> Repository<K, T>
   where K: Hash + Eq
{
   pub fn load(&mut self, key: K) -> Result<Cache<T>> {
      let Some(arc) = self.pool.get(key) else { bail!("not yet implemented."); };

      let cache = Cache::new(arc);
      Ok(cache)
   }
}

#[cfg(feature="jvm")]
impl<K, T> Repository<K, T>
   where K: Hash + Eq,
         T: ConvertJava
{
   pub fn new(env: &mut JNIEnv) -> Self {
      Repository {
         pool: UniqueCachePool::new(env)
      }
   }

   pub fn save(&mut self, key: K, value: T) -> Cache<T> {
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }
}

#[cfg(not(feature="jvm"))]
impl<K, T> Repository<K, T>
   where K: Hash + Eq
{
   pub fn new() -> Self {
      Repository {
         pool: UniqueCachePool::new()
      }
   }

   pub fn save(&mut self, key: K, value: T) -> Cache<T> {
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::JObject;

   use super::Repository;

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_saveLoad(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::new(&mut env);
      repository.save("A".to_string(), 42);

      {
         let result = repository.load("A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         let cache_lock = cache.lock().unwrap();
         assert_eq!(42, **cache_lock);
      }

      repository.save("A".to_string(), 13);

      {
         let result = repository.load("A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         let cache_lock = cache.lock().unwrap();
         assert_eq!(13, **cache_lock);
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_load_1noSuchCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<_, i32>::new(&mut env);

      let result = repository.load("A".to_string());
      assert!(result.is_err());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_save_1viaCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::new(&mut env);
      repository.save("A".to_string(), 42);

      {
         let cache = repository.load("A".to_string()).unwrap();
         let mut cache_lock = cache.lock().unwrap();
         assert_eq!(42, **cache_lock);
         cache_lock.save(13);
      }

      {
         let cache = repository.load("A".to_string()).unwrap();
         let cache_lock = cache.lock().unwrap();
         assert_eq!(13, **cache_lock);
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_save_1affectAnotherCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::new(&mut env);
      repository.save("A".to_string(), 42);

      let cache1 = repository.load("A".to_string()).unwrap();
      {
         let mut cache1_lock = cache1.lock().unwrap();
         assert_eq!(42, **cache1_lock);
      }

      repository.save("A".to_string(), 13);

      let cache2 = repository.load("A".to_string()).unwrap();
      {
         let cache1_lock = cache1.lock().unwrap();
         assert_eq!(13, **cache1_lock);
      }
      {
         let mut cache2_lock = cache2.lock().unwrap();
         assert_eq!(13, **cache2_lock);
         cache2_lock.save(0);
      }

      {
         let cache1_lock = cache1.lock().unwrap();
         assert_eq!(0, **cache1_lock);
      }
      {
         let cache2_lock = cache2.lock().unwrap();
         assert_eq!(0, **cache2_lock);
      }
   }
}
