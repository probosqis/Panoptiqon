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

use serde::de::Visitor;
use serde::Deserializer;

pub struct CacheDeserializer {
}

impl<'de> Deserializer<'de> for CacheDeserializer
{
   type Error = !;

   fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_unit_struct<V>(
      self,
      name: &'static str,
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_newtype_struct<V>(
      self,
      name: &'static str,
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_tuple_struct<V>(
      self,
      name: &'static str,
      len: usize,
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_struct<V>(
      self,
      name: &'static str,
      fields: &'static [&'static str],
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_enum<V>(
      self,
      name: &'static str,
      variants: &'static [&'static str],
      visitor: V,
   ) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
      where V: Visitor<'de>
   {
      todo!();
   }

   #[inline]
   fn is_human_readable(&self) -> bool {
      todo!();
   }
}

pub(crate) struct Loader {
}

impl Loader {
   pub(crate) fn new() -> Self {
      Self {
      }
   }
}
