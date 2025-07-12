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

use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, RwLock};
use serde::de::Visitor;
use serde::Deserializer;
use serde_json::de::IoRead;
use crate::repository::DynRepository;

pub struct CacheDeserializer {
   deserializer: serde_json::Deserializer<IoRead<BufReader<File>>>
}

impl CacheDeserializer {
   pub(crate) fn new(
      json_deserializer: serde_json::Deserializer<IoRead<BufReader<File>>>
   ) -> Self {
      Self {
         deserializer: json_deserializer
      }
   }
}

impl<'de> Deserializer<'de> for &mut CacheDeserializer {
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

   #[cfg(test)]
   pub(crate) fn repositories(&self) -> Vec<Arc<dyn DynRepository>> {
      self.repositories.read().unwrap().clone()
   }
}
