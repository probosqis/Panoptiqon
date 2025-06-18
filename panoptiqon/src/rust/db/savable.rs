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

use std::path::{Path, PathBuf};
use serde::Serialize;
use crate::cache::CacheContent;
use crate::db::saver;
use crate::unique_cache::UniqueCache;

pub(crate) trait Savable {
   fn file_path(&self, dir_path: &Path) -> PathBuf;

   fn serialize(
      &self,
      serializer: saver::Serializer
   ) -> anyhow::Result<
      <saver::Serializer as serde::Serializer>::Ok,
      <saver::Serializer as serde::Serializer>::Error
   >;
}

impl<T> Savable for UniqueCache<T>
   where T: CacheContent + Serialize
{
   fn file_path(&self, dir_path: &Path) -> PathBuf {
      self.get().file_path(dir_path)
   }

   fn serialize(
      &self,
      serializer: saver::Serializer
   ) -> anyhow::Result<
      <saver::Serializer as serde::Serializer>::Ok,
      <saver::Serializer as serde::Serializer>::Error
   > {
      self.get().serialize(serializer)
   }
}
