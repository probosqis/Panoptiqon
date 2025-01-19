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

use jni::objects::JObject;
use crate::jvm_type::{jvm_type, JvmType};

pub struct JvmNullable<T: JvmType>(pub T);

impl<T: JvmType> JvmType for JvmNullable<T> {
   fn j_object(&self) -> &JObject {
      self.0.j_object()
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
