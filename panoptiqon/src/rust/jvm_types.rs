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

#![cfg(feature = "jvm")]

use std::marker::PhantomData;
use jni::objects::JObject;
use crate::jvm_type;
use crate::jvm_type::JvmType;

pub struct JvmNullable<'local, T>(
   T,
   PhantomData<&'local ()>
) where T: JvmType<'local> + 'local;

impl<'local, T> JvmType<'local> for JvmNullable<'local, T>
   where T: JvmType<'local>
{
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmNullable<'local, T> {
      JvmNullable(
         T::from_j_object(j_object),
         PhantomData
      )
   }

   fn j_object(&self) -> &JObject<'local> {
      self.0.j_object()
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0.into_j_object()
   }
}

jvm_type! {
   JvmCache,
   JvmBoolean,
   JvmByte,
   JvmShort,
   JvmInteger,
   JvmLong,
   JvmFloat,
   JvmDouble,
   JvmString,
   JvmUnit,
   JvmPair,
   JvmTriple,
   JvmList,
}
