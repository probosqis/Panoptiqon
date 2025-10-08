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

use std::borrow::Borrow;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
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
   db_scheduler: Arc<DbScheduler>,
   dir_path: saver::DirPath,
   map: Mutex<FnvHashMap<T::Key, Arc<UniqueCache<T>>>>
}

#[cfg(not(feature="jvm"))]
pub(crate) struct UniqueCachePool<T: CacheContent> {
   db_scheduler: Arc<DbScheduler>,
   dir_path: saver::DirPath,
   map: Mutex<FnvHashMap<T::Key, Arc<UniqueCache<T>>>>
}

impl<T: CacheContent> UniqueCachePool<T> {
   pub fn get<Q>(&self, key: &Q) -> Option<Arc<UniqueCache<T>>>
      where T::Key: Borrow<Q>,
            Q: Hash + Eq + ?Sized
   {
      self.map.lock().ok()?.get(key).map(|arc| arc.clone())
   }

   pub(crate) fn dir_path(&self) -> &saver::DirPath {
      &self.dir_path
   }
}

#[cfg(feature="jvm")]
impl<T: CacheContent> UniqueCachePool<T> {
   pub fn new(
      env: &mut JNIEnv,
      db_scheduler: Arc<DbScheduler>,
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
         map: Mutex::new(FnvHashMap::default())
      }
   }

   pub fn update(&self, key: T::Key, value: T) -> Arc<UniqueCache<T>>
      where T: for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
               + CloneIntoJvmHelper
               + Serialize
   {
      use std::collections::hash_map::Entry;

      let mut lock = self.map.lock().expect("Cache pool was poisoned");
      let entry = lock.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            arc.save(value);
            Arc::clone(arc)
         }
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_saved(
               &self.jvm,
               Arc::clone(&self.jvm_unique_cache_refs),
               Arc::clone(&self.db_scheduler),
               &self.dir_path,
               value
            );
            entry.insert(Arc::clone(&arc));
            arc
         }
      }
   }

   pub fn insert_if_vacant(
      &self,
      key: T::Key,
      value: T
   ) -> Arc<UniqueCache<T>>
      where T: for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
         + CloneIntoJvmHelper
   {
      use std::collections::hash_map::Entry;

      let mut lock = self.map.lock().expect("Cache pool was poisoned");
      let entry = lock.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            Arc::clone(arc)
         },
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_not_saved(
               &self.jvm,
               Arc::clone(&self.jvm_unique_cache_refs),
               Arc::clone(&self.db_scheduler),
               &self.dir_path,
               value
            );
            entry.insert(Arc::clone(&arc));
            arc
         }
      }
   }
}

#[cfg(not(feature="jvm"))]
impl<T: CacheContent> UniqueCachePool<T> {
   pub fn new(
      db_scheduler: Arc<DbScheduler>,
      dir_path: &saver::DirPath
   ) -> Self {
      UniqueCachePool {
         db_scheduler,
         dir_path: saver::DirPath::clone(dir_path),
         map: Mutex::new(FnvHashMap::default())
      }
   }

   pub fn update(&self, key: T::Key, value: T) -> Arc<UniqueCache<T>>
      where T: Serialize
   {
      use std::collections::hash_map::Entry;

      let mut lock = self.map.lock().expect("Cache pool was poisoned");
      let entry = lock.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            arc.save(value);
            Arc::clone(arc)
         }
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_saved(
               Arc::clone(&self.db_scheduler),
               &self.dir_path,
               value
            );
            entry.insert(Arc::clone(&arc));
            arc
         }
      }
   }

   /// 指定されたkeyがまだこのPool内に存在しない場合追加する。
   /// すでに存在する場合はそのUniqueCacheを返却する。
   ///
   /// いずれの場合も、[Self::new]と異なり[DbScheduler]への
   /// [push][DbScheduler::push]は行わない。
   pub fn insert_if_vacant(
      &self,
      key: T::Key,
      value: T
   ) -> Arc<UniqueCache<T>> {
      use std::collections::hash_map::Entry;

      let mut lock = self.map.lock().expect("Cache pool was poisoned");
      let entry = lock.entry(key);

      match entry {
         Entry::Occupied(entry) => {
            let arc = entry.get();
            Arc::clone(arc)
         },
         Entry::Vacant(entry) => {
            let arc = UniqueCache::new_not_saved(
               Arc::clone(&self.db_scheduler),
               &self.dir_path,
               value
            );
            entry.insert(Arc::clone(&arc));
            arc
         }
      }
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::{Path, PathBuf};
   use serde::Serialize;
   use crate::cache::CacheContent;

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct CacheContentImpl(String, i32);

   impl CacheContent for CacheContentImpl {
      type Key = String;

      fn key(&self) -> &String {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &String) -> PathBuf {
         dir_path.join(key)
      }
   }

   #[test]
   fn insert_if_vacant() {
      use std::cell::RefCell;
      use std::sync::{Arc, ReentrantLock};
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::{DirPath, Saver};
      use crate::db::scheduler::DbScheduler;
      use crate::pool::UniqueCachePool;

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
      let mut pool = UniqueCachePool::new(
         Arc::new(DbScheduler::new(Saver::new(in_memory_db))),
         &DirPath::new(PathBuf::from("test/UniqueCachePool/insert_if_vacant"))
      );

      assert!(pool.get("A").is_none());

      let content = CacheContentImpl("A".to_string(), 0);
      pool.insert_if_vacant("A".to_string(), content);

      assert!(pool.get("A").is_some());
      assert_eq!(
         CacheContentImpl("A".to_string(), 0),
         *pool.get("A").unwrap().get()
      );

      let content = CacheContentImpl("A".to_string(), 1);
      pool.insert_if_vacant("A".to_string(), content);

      assert!(pool.get("A").is_some());
      assert_eq!(
         CacheContentImpl("A".to_string(), 0),
         *pool.get("A").unwrap().get()
      );
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use std::path::{Path, PathBuf};
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::Serialize;
   use crate::cache::CacheContent;
   use crate::convert_jvm::CloneIntoJvm;
   use crate::db::scheduler::DbScheduler;
   use crate::jvm_type;
   use crate::jvm_types::JvmString;
   use super::UniqueCachePool;

   jvm_type! {
      JvmContent,
   }

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct Content(String, i32);

   impl CacheContent for Content {
      type Key = String;
      type JvmKey<'local> = JvmString<'local>;
      type JvmType<'local> = JvmContent<'local>;

      fn key(&self) -> &String {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &String) -> PathBuf {
         dir_path.join(key)
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmContent<'local>> for Content {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmContent<'local> {
         use crate::jvm_type::JvmType;

         let j_object = self.0.clone_into_jvm(env);
         JvmContent(j_object.into_j_object())
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_createCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::cell::RefCell;
      use std::path::PathBuf;
      use std::sync::{Arc, ReentrantLock};
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::{self, Saver};

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
      let db_scheduler = Arc::new(DbScheduler::new(Saver::new(in_memory_db)));
      let pool = UniqueCachePool::new(
         &mut env,
         db_scheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/createCache"))
      );

      let unique_cache = pool.update("A".to_string(), Content("A".to_string(), 42));
      assert_eq!(Content("A".to_string(), 42), *unique_cache.get());

      let unique_cache = pool.update("B".to_string(), Content("B".to_string(), 43));
      assert_eq!(Content("B".to_string(), 43), *unique_cache.get());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_getCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::cell::RefCell;
      use std::path::PathBuf;
      use std::sync::{Arc, ReentrantLock};
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::{self, Saver};

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
      let db_scheduler = Arc::new(DbScheduler::new(Saver::new(in_memory_db)));
      let pool = UniqueCachePool::new(
         &mut env,
         db_scheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/getCache"))
      );

      let _ = pool.update("A".to_string(), Content("A".to_string(), 42));

      let unique_cache = pool.get("A");
      assert!(unique_cache.is_some());
      let unique_cache = unique_cache.unwrap();
      assert_eq!(Content("A".to_string(), 42), *unique_cache.get());

      let unique_cache = pool.get("B");
      assert!(unique_cache.is_none());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_pooling(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      use std::cell::RefCell;
      use std::path::PathBuf;
      use std::sync::{Arc, ReentrantLock};
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::{self, Saver};

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
      let db_scheduler = Arc::new(DbScheduler::new(Saver::new(in_memory_db)));
      let pool = UniqueCachePool::new(
         &mut env,
         db_scheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/pooling"))
      );
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
      use std::cell::RefCell;
      use std::path::PathBuf;
      use std::sync::{Arc, ReentrantLock};
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::{self, Saver};

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
      let db_scheduler = Arc::new(DbScheduler::new(Saver::new(in_memory_db)));
      let pool = UniqueCachePool::new(
         &mut env,
         db_scheduler,
         &saver::DirPath::new(PathBuf::from("test/CachePoolTest/save"))
      );
      let unique_cache = pool.update("A".to_string(), Content("A".to_string(), 42));

      assert_eq!(Content("A".to_string(), 42), *unique_cache.get());
      unique_cache.save(Content("A".to_string(), 43));
      assert_eq!(Content("A".to_string(), 43), *unique_cache.get());
   }
}
