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
   specialization
)]

use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::cache::CacheContent;
use crate::db::loader::Loader;
use crate::db::scheduler::DbScheduler;
use crate::repository::Repository;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   crate::convert_jvm::CloneIntoJvmHelper,
};

pub mod cache;
pub mod repository;

pub(crate) mod db;
mod pool;
mod unique_cache;

#[cfg(feature = "jvm")]
pub mod convert_jvm;
#[cfg(feature = "jvm")]
pub mod jvm_type;
#[cfg(feature = "jvm")]
pub mod jvm_types;

pub struct Panoptiqon {
   db_scheduler: Arc<DbScheduler>,
   loader: Arc<Loader>
}

impl Panoptiqon {
   pub fn new() -> Self {
      use crate::db::saver::Saver;

      Panoptiqon {
         db_scheduler: Arc::new(DbScheduler::new(Saver::new())),
         loader: Arc::new(Loader::new())
      }
   }

   #[cfg(not(feature = "jvm"))]
   pub fn new_repository<T>(
      &self,
      dir_path: impl AsRef<Path>
   ) -> Arc<Mutex<Repository<T>>>
      where T: CacheContent
   {
      Arc::new(Mutex::new(
         Repository::new(Arc::clone(&self.db_scheduler), dir_path)
      ))
   }

   #[cfg(feature = "jvm")]
   pub fn new_repository<T>(
      &self,
      env: &mut JNIEnv,
      dir_path: impl AsRef<Path>
   ) -> Arc<Mutex<Repository<T>>>
      where T: CacheContent + CloneIntoJvmHelper
   {
      let repository = Arc::new(Mutex::new(
         Repository::new(env, Arc::clone(&self.db_scheduler), dir_path)
      ));

      let dyn_repository: Arc<Mutex<Repository<T>>> = Arc::clone(&repository);
      self.loader.push_repository(dyn_repository);

      repository
   }
}
