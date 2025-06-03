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

use std::sync::Arc;
use fnv::FnvHashMap;
use serde::Serialize;
use crate::cache::CacheContent;
use crate::db::saver;
use crate::db::scheduler::DbScheduler;
use crate::unique_cache::UniqueCache;

#[cfg(feature="jvm")]
use {
   jni::JavaVM,
   jni::JNIEnv,
   crate::convert_jvm::{CloneIntoJvm, CloneIntoJvmHelper},
   crate::unique_cache::JvmUniqueCacheRefs,
};

#[cfg(feature="jvm")]
pub(crate) struct UniqueCachePool<T: CacheContent> {
   jvm: JavaVM,
   jvm_unique_cache_refs: Arc<JvmUniqueCacheRefs>,
   db_scheduler: &'static DbScheduler,
   dir_path: saver::DirPath,
   map: FnvHashMap<T::Key, Arc<UniqueCache<T>>>
}

#[cfg(not(feature="jvm"))]
pub(crate) struct UniqueCachePool<T: CacheContent> {
   db_scheduler: &'static DbScheduler,
   dir_path: saver::DirPath,
   map: FnvHashMap<T::Key, Arc<UniqueCache<T>>>
}

impl<T: CacheContent> UniqueCachePool<T> {
   pub fn get(&self, key: T::Key) -> Option<Arc<UniqueCache<T>>> {
      self.map.get(&key).map(|arc| arc.clone())
   }
}

#[cfg(feature="jvm")]
impl<T: CacheContent> UniqueCachePool<T> {
   pub fn new(
      env: &mut JNIEnv,
      db_scheduler: &'static DbScheduler,
      dir_path: &saver::DirPath
   ) -> Self
      where T: CloneIntoJvmHelper
   {
      let jvm = env.get_java_vm().unwrap();

      UniqueCachePool {
         jvm,
         jvm_unique_cache_refs: Arc::new(T::get_jvm_refs(env)),
         db_scheduler,
         dir_path: saver::DirPath::clone(dir_path),
         map: FnvHashMap::default()
      }
   }

   pub fn update(&mut self, key: T::Key, value: T) -> Arc<UniqueCache<T>>
      where T: for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
               + CloneIntoJvmHelper
               + Serialize
               + Send + Sync
               + 'static
   {
      use std::collections::hash_map::Entry;

      let entry = self.map.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            arc.save(value);
            Arc::clone(arc)
         }
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_saved(
               &self.jvm,
               self.jvm_unique_cache_refs.clone(),
               self.db_scheduler,
               &self.dir_path,
               value
            );
            entry.insert(arc.clone());
            arc
         }
      }
   }
}

#[cfg(not(feature="jvm"))]
impl<T: CacheContent> UniqueCachePool<T> {
   pub fn new(
      db_scheduler: &'static DbScheduler,
      dir_path: &saver::DirPath
   ) -> Self {
      UniqueCachePool {
         db_scheduler,
         dir_path: saver::DirPath::clone(dir_path),
         map: FnvHashMap::default()
      }
   }

   pub fn update(&mut self, key: T::Key, value: T) -> Arc<UniqueCache<T>>
      where T: Serialize + Send + Sync + 'static
   {
      use std::collections::hash_map::Entry;

      let entry = self.map.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            arc.save(value);
            Arc::clone(arc)
         }
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_saved(
               self.db_scheduler,
               &self.dir_path,
               value
            );
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
   use serde::Serialize;
   use crate::cache::CacheContent;
   use crate::convert_jvm::CloneIntoJvm;
   use crate::db::scheduler::DbScheduler;
   use crate::jvm_type;
   use super::UniqueCachePool;

   jvm_type! {
      JvmContent,
   }

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct Content(String, i32);

   impl CacheContent for Content {
      type Key = String;
      type JvmType<'local> = JvmContent<'local>;

      fn key(&self) -> String {
         self.0.clone()
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmContent<'local>> for Content {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmContent<'local> {
         use crate::jvm_type::JvmType;

         let j_object = self.0.clone_into_jvm(env);
         JvmContent(j_object.into_j_object())
      }
   }

   #[allow(non_upper_case_globals)]
   static createCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_createCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::path::PathBuf;
      use crate::db::saver;

      let mut pool = UniqueCachePool::new(
         &mut env,
         &createCache_dbScheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/createCache"))
      );

      let unique_cache = pool.update("A".to_string(), Content("A".to_string(), 42));
      assert_eq!(Content("A".to_string(), 42), *unique_cache.get());

      let unique_cache = pool.update("B".to_string(), Content("B".to_string(), 43));
      assert_eq!(Content("B".to_string(), 43), *unique_cache.get());
   }

   #[allow(non_upper_case_globals)]
   static getCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_getCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::path::PathBuf;
      use crate::db::saver;

      let mut pool = UniqueCachePool::new(
         &mut env,
         &getCache_dbScheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/getCache"))
      );

      let _ = pool.update("A".to_string(), Content("A".to_string(), 42));

      let unique_cache = pool.get("A".to_string());
      assert!(unique_cache.is_some());
      let unique_cache = unique_cache.unwrap();
      assert_eq!(Content("A".to_string(), 42), *unique_cache.get());

      let unique_cache = pool.get("B".to_string());
      assert!(unique_cache.is_none());
   }

   #[allow(non_upper_case_globals)]
   static pooling_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_pooling(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::path::PathBuf;
      use std::sync::Arc;
      use crate::db::saver;

      let mut pool = UniqueCachePool::new(
         &mut env,
         &pooling_dbScheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/pooling"))
      );
      let cache1_ptr = Arc::as_ptr(&pool.update("A".to_string(), Content("A".to_string(), 42))) as *const _;
      let cache2_ptr = Arc::as_ptr(&pool.update("A".to_string(), Content("A".to_string(), 42))) as *const _;
      let cache3_ptr = Arc::as_ptr(&pool.update("B".to_string(), Content("B".to_string(), 42))) as *const _;

      assert_eq!(cache1_ptr, cache2_ptr);
      assert_ne!(cache1_ptr, cache3_ptr);
   }

   #[allow(non_upper_case_globals)]
   static save_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_save(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::path::PathBuf;
      use crate::db::saver;

      let mut pool = UniqueCachePool::new(
         &mut env,
         &save_dbScheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/save"))
      );
      let unique_cache = pool.update("A".to_string(), Content("A".to_string(), 42));

      assert_eq!(Content("A".to_string(), 42), *unique_cache.get());
      unique_cache.save(Content("A".to_string(), 43));
      assert_eq!(Content("A".to_string(), 43), *unique_cache.get());
   }
}
