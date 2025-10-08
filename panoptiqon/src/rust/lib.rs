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

#![allow(incomplete_features)]
#![feature(
   get_mut_unchecked, mapped_lock_guards, never_type, ptr_metadata,
   reentrant_lock, specialization
)]

use std::cell::RefCell;
use std::path::Path;
use std::sync::{Arc, ReentrantLock};
use crate::cache::CacheContent;
use crate::db::loader::Loader;
use crate::db::scheduler::DbScheduler;
use crate::repository::Repository;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   serde::{Deserialize, Serialize},
   crate::convert_jvm::{CloneFromJvm, CloneIntoJvm, CloneIntoJvmHelper},
   crate::jvm_types::{JvmCache, JvmCacheId, JvmErased},
};

#[cfg(any(test, feature = "testable"))]
use {
   crate::db::in_memory_db::InMemoryDb,
};

pub mod cache;
pub mod repository;

pub(crate) mod db;
pub(crate) mod dyn_repository;
mod pool;
mod unique_cache;

#[cfg(feature = "jvm")]
pub mod convert_jvm;
#[cfg(feature = "jvm")]
pub mod jvm_repository_creator;
#[cfg(feature = "jvm")]
pub mod jvm_type;
#[cfg(feature = "jvm")]
pub mod jvm_types;

pub struct Panoptiqon {
   #[cfg(any(test, feature = "testable"))]
   in_memory_db: Arc<ReentrantLock<RefCell<InMemoryDb>>>,
   db_scheduler: Arc<DbScheduler>,
   loader: Arc<Loader>
}

impl Panoptiqon {
   pub fn new() -> Self {
      use crate::db::saver::Saver;

      #[cfg(any(test, feature = "testable"))]
      {
         let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
         let saver = Saver::new(Arc::clone(&in_memory_db));
         Panoptiqon {
            in_memory_db,
            db_scheduler: Arc::new(DbScheduler::new(saver)),
            loader: Arc::new(Loader::new())
         }
      }

      #[cfg(not(any(test, feature = "testable")))]
      {
         Panoptiqon {
            db_scheduler: Arc::new(DbScheduler::new(Saver::new())),
            loader: Arc::new(Loader::new())
         }
      }
   }

   #[cfg(not(feature = "jvm"))]
   pub fn new_repository<T>(
      &self,
      dir_path: impl AsRef<Path>
   ) -> Arc<Repository<T>>
   where
      T: CacheContent
   {
      let repository = Arc::new(
         Repository::new(
            Arc::clone(&self.db_scheduler),
            Arc::downgrade(&self.loader),
            dir_path,
            #[cfg(any(test, feature = "testable"))]
            Arc::clone(&self.in_memory_db)
         )
      );

      let dyn_repository = Arc::clone(&repository);
      self.loader.push_repository(dyn_repository);

      repository
   }

   #[cfg(feature = "jvm")]
   pub fn new_repository<T>(
      &self,
      env: &mut JNIEnv,
      dir_path: impl AsRef<Path>
   ) -> Arc<Repository<T>>
   where
      T: CacheContent
         + CloneIntoJvmHelper
         + Serialize
         + for<'de> Deserialize<'de>
         + for<'a> CloneIntoJvm<'a, T::JvmType<'a>>
         + for<'a> CloneFromJvm<'a, T::JvmType<'a>>,
      T::Key: for<'a> CloneFromJvm<'a, T::JvmKey<'a>>
   {
      let repository = Arc::new(
         Repository::new(
            env,
            Arc::clone(&self.db_scheduler),
            Arc::downgrade(&self.loader),
            dir_path,
            #[cfg(any(test, feature = "testable"))]
            Arc::clone(&self.in_memory_db)
         )
      );

      let dyn_repository = Arc::clone(&repository);
      self.loader.push_repository(dyn_repository);

      repository
   }

   #[cfg(feature = "jvm")]
   pub fn load_jvm<'local>(
      &self,
      env: &mut JNIEnv<'local>,
      cache_id: &JvmCacheId<'local>
   ) -> anyhow::Result<JvmCache<'local, JvmErased<'local>>> {
      self.loader.load_jvm(env, cache_id)
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::{Path, PathBuf};
   use serde::Serialize;
   use crate::cache::CacheContent;
   use super::Panoptiqon;

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct CacheContentImpl(i32, i32);

   impl CacheContent for CacheContentImpl {
      type Key = i32;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   #[allow(non_snake_case)]
   #[test]
   fn newRepository_pushesIntoLoader() {
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;

      let panoptiqon = Panoptiqon::new();
      let repository = panoptiqon.new_repository::<CacheContentImpl>(
         "test/Panoptiqon/newRepository_pushesIntoLoader"
      );

      assert_eq!(
         Arc::as_ptr(&panoptiqon.loader),
         Weak::as_ptr(repository.loader())
      );

      let repositories = panoptiqon.loader.repositories();
      assert_eq!(1, repositories.len());
      assert_eq!(
         Arc::as_ptr(&repositories[0]) as *const Repository<CacheContentImpl>,
         Arc::as_ptr(&repository)
      );
   }
}
