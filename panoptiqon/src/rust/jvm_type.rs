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

use jni::objects::JObject;

pub trait JvmType<'local> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> Self where Self: 'local;
   fn j_object(&self) -> &JObject<'local>;
   fn into_j_object(self) -> JObject<'local>;
}

#[macro_export]
macro_rules! jvm_type {
   () => {
   };
   ($($type_name:ident),+ $(,)?) => {
      $(
         #[repr(transparent)]
         pub struct $type_name<'local>(
            ::jni::objects::JObject<'local>
         );

         impl<'local> $crate::jvm_type::JvmType<'local> for $type_name<'local> {
            unsafe fn from_j_object(
               j_object: ::jni::objects::JObject<'local>
            ) -> $type_name<'local> {
               $type_name(j_object)
            }

            fn j_object(&self) -> &::jni::objects::JObject<'local> {
               &self.0
            }

            fn into_j_object(self) -> ::jni::objects::JObject<'local> {
               self.0
            }
         }
      )+
   };
}
