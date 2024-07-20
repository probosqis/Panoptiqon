/*
 * Copyright 2024 wcaokaze
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
use std::ops::{Deref, DerefMut};

#[cfg(feature="jvm")]
use jni::objects::GlobalRef;

pub struct Cache<T> {
   #[cfg(feature="jvm")]
   jvm_state: GlobalRef,
   value: T
}

impl<T> Cache<T> {
   #[cfg(feature="jvm")]
   pub(crate) fn new(jvm_state: GlobalRef, initial_value: T) -> Self {
      Cache {
         jvm_state,
         value: initial_value
      }
   }

   #[cfg(not(feature="jvm"))]
   pub(crate) fn new(initial_state: T) -> Self {
      Cache {
         value: initial_state
      }
   }
}

impl<T> Deref for Cache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
   }
}

impl<T> DerefMut for Cache<T> {
   fn deref_mut(&mut self) -> &mut T {
      &mut self.value
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::JObject;

   use super::Cache;

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_deref(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let jvm_state = env
         .new_object("com/wcaokaze/probosqis/panoptiqon/CacheInternal", "()V", &[])
         .unwrap();
      let jvm_state = env.new_global_ref(jvm_state).unwrap();
      let mut cache = Cache::new(jvm_state, 42);
      assert_eq!(42, cache.value);

      assert_eq!(42, *cache);

      *cache = 43;
      assert_eq!(43, cache.value);
   }
}
