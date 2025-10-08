/*
 * Copyright 2025 wcaokaze
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

use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, RwLock};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use serde_json::de::IoRead;
use crate::cache::{Cache, CacheContent};
use crate::dyn_repository::DynRepository;
use crate::Repository;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   crate::convert_jvm::{CloneIntoJvm, CloneIntoJvmHelper},
   crate::jvm_types::{JvmCache, JvmCacheId, JvmErased},
};

pub struct CacheDeserializer<'a, R: Read> {
   pub(crate) deserializer: serde_json::Deserializer<IoRead<BufReader<R>>>,
   pub(crate) loader: &'a Loader
}

impl<'a, R: Read> CacheDeserializer<'a, R> {
   pub(crate) fn new(
      json_deserializer: serde_json::Deserializer<IoRead<BufReader<R>>>,
      loader: &'a Loader
   ) -> Self {
      Self {
         deserializer: json_deserializer,
         loader
      }
   }
}

impl<'de, 'a, R: Read> Deserializer<'de> for &mut CacheDeserializer<'a, R> {
   type Error = serde_json::Error;

   fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_any(visitor)
   }

   fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_bool(visitor)
   }

   fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_i8(visitor)
   }

   fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_i16(visitor)
   }

   fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_i32(visitor)
   }

   fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_i64(visitor)
   }

   fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_i128(visitor)
   }

   fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_u8(visitor)
   }

   fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_u16(visitor)
   }

   fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_u32(visitor)
   }

   fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_u64(visitor)
   }

   fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_u128(visitor)
   }

   fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_f32(visitor)
   }

   fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_f64(visitor)
   }

   fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_char(visitor)
   }

   fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_str(visitor)
   }

   fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_string(visitor)
   }

   fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_bytes(visitor)
   }

   fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_byte_buf(visitor)
   }

   fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_option(visitor)
   }

   fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_unit(visitor)
   }

   fn deserialize_unit_struct<V>(
      self,
      name: &'static str,
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_unit_struct(name, visitor)
   }

   fn deserialize_newtype_struct<V>(
      self,
      name: &'static str,
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_newtype_struct(name, visitor)
   }

   fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_seq(visitor)
   }

   fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_tuple(len, visitor)
   }

   fn deserialize_tuple_struct<V>(
      self,
      name: &'static str,
      len: usize,
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_tuple_struct(name, len, visitor)
   }

   fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_map(visitor)
   }

   fn deserialize_struct<V>(
      self,
      name: &'static str,
      fields: &'static [&'static str],
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_struct(name, fields, visitor)
   }

   fn deserialize_enum<V>(
      self,
      name: &'static str,
      variants: &'static [&'static str],
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_enum(name, variants, visitor)
   }

   fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_identifier(visitor)
   }

   fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      self.deserializer.deserialize_ignored_any(visitor)
   }
}

pub(crate) struct Loader {
   repositories: RwLock<Vec<Arc<dyn DynRepository>>>
}

impl Loader {
   pub(crate) fn new() -> Self {
      Self {
         repositories: RwLock::new(Vec::new())
      }
   }

   pub(crate) fn push_repository(&self, repository: Arc<dyn DynRepository>) {
      let mut lock = self.repositories.write().unwrap();
      lock.push(repository);
   }

   fn find_repository<'a, T: CacheContent>(
      repositories: &'a [Arc<dyn DynRepository>],
      repository_dir_path: &Path
   ) -> anyhow::Result<&'a Repository<T>> {
      use std::any;
      use anyhow::Context;

      let repository = repositories.iter()
         .filter_map(|repo| repo.downcast::<T>())
         .find(|repo| repo.dir_path() == repository_dir_path)
         .context(format!(
            "Repository not found (repo dir: {}, content type: {})",
            repository_dir_path.display(), any::type_name::<T>()
         ))?;

      Ok(repository)
   }

   #[cfg(not(feature = "jvm"))]
   pub(crate) fn load<T>(
      &self,
      repository_dir_path: impl AsRef<Path>,
      file_path: impl AsRef<Path>
   ) -> anyhow::Result<Cache<T>>
      where for<'de> T: CacheContent + Deserialize<'de>
   {
      let lock = self.repositories.read()
         .map_err(|_| anyhow::anyhow!("Loader is poisoned"))?;
      let repository_lock = Self::find_repository(&*lock, repository_dir_path.as_ref())?;
      repository_lock.load_file(file_path)
   }

   #[cfg(feature = "jvm")]
   pub(crate) fn load<T>(
      &self,
      repository_dir_path: impl AsRef<Path>,
      file_path: impl AsRef<Path>
   ) -> anyhow::Result<Cache<T>>
      where for<'de, 'local> T: Deserialize<'de>
            + CloneIntoJvm<'local, T::JvmType<'local>>
            + CloneIntoJvmHelper
   {
      let lock = self.repositories.read()
         .map_err(|_| anyhow::anyhow!("Loader is poisoned"))?;
      let repository_lock = Self::find_repository(&*lock, repository_dir_path.as_ref())?;
      repository_lock.load_file(file_path)
   }

   #[cfg(feature = "jvm")]
   pub(crate) fn load_jvm<'local>(
      &self,
      env: &mut JNIEnv<'local>,
      cache_id: &JvmCacheId<'local>
   ) -> anyhow::Result<JvmCache<'local, JvmErased<'local>>> {
      use anyhow::Context;
      use crate::cache::CacheId;
      use crate::convert_jvm::CloneFromJvm;

      let cache_id = CacheId::clone_from_jvm(env, cache_id);

      let lock = self.repositories.read()
         .map_err(|_| anyhow::anyhow!("Loader is poisoned"))?;
      let repository = lock.iter()
         .find(|repo| repo.dir_path() == cache_id.repository_dir_path)
         .context(format!(
            "Repository not found (repo dir: {})",
            cache_id.repository_dir_path.display()
         ))?;
      repository.load_file_jvm(env, &cache_id.file_path)
   }

   #[cfg(any(test, feature = "testable"))]
   pub(crate) fn repositories(&self) -> Vec<Arc<dyn DynRepository>> {
      self.repositories.read().unwrap().clone()
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::{Path, PathBuf};
   use serde::{Deserialize, Serialize};
   use crate::cache::CacheContent;
   use super::Loader;

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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

   #[test]
   fn load() {
      use std::fs;
      use scopeguard::defer;
      use std::cell::RefCell;
      use std::sync::{Arc, ReentrantLock};
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;
      use crate::repository::Repository;

      fs::create_dir_all("test/Loader/load").unwrap();
      fs::write("test/Loader/load/0", "[0,42]").unwrap();

      defer! {
         fs::remove_dir_all("test/Loader/load").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(in_memory_db)));
      let saver = Saver::new(Arc::clone(&in_memory_db));
      let repository = Arc::new(
         Repository::<CacheContentImpl>::new(
            Arc::new(DbScheduler::new(saver)),
            Arc::downgrade(&loader),
            "test/Loader/load",
            in_memory_db
         )
      );

      let dyn_repo = Arc::clone(&repository);
      loader.push_repository(dyn_repo);

      let cache = loader.load("test/Loader/load", "test/Loader/load/0").unwrap();

      assert_eq!(
         CacheContentImpl(0, 42),
         *cache.get()
      );
   }

   #[allow(non_snake_case)]
   #[test]
   fn load_repositoryNotFound() {
      use std::cell::RefCell;
      use std::fs;
      use std::sync::{Arc, ReentrantLock};
      use scopeguard::defer;
      use crate::cache::Cache;
      use crate::db::in_memory_db::InMemoryDb;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;
      use crate::repository::Repository;

      fs::create_dir_all("test/Loader/load_repositoryNotFound").unwrap();
      fs::write("test/Loader/load_repositoryNotFound/0", "[0,42]").unwrap();

      defer! {
         fs::remove_dir_all("test/Loader/load_repositoryNotFound").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
      let saver = Saver::new(Arc::clone(&in_memory_db));
      let repository = Arc::new(
         Repository::<CacheContentImpl>::new(
            Arc::new(DbScheduler::new(saver)),
            Arc::downgrade(&loader),
            "test/Loader/load_repositoryNotFound_dummy",
            in_memory_db
         )
      );

      let dyn_repo = Arc::clone(&repository);
      loader.push_repository(dyn_repo);

      let result: anyhow::Result<Cache<CacheContentImpl>> = loader.load(
         "test/Loader/load_repositoryNotFound",
         "test/Loader/load_repositoryNotFound/0"
      );

      assert!(result.is_err());
   }
}
