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
use std::ops::Deref;

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   jni::JavaVM,
   jni::objects::{GlobalRef, JMethodID, JValueGen},
   jni::signature::{Primitive, ReturnType}
};

#[cfg(feature="jvm")]
pub struct UniqueCache<T: ConvertJava> {
   jvm_state: GlobalRef,
   jvm: JavaVM,
   update_method_id: JMethodID,
   value: T
}

#[cfg(not(feature="jvm"))]
pub struct UniqueCache<T> {
   value: T
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> UniqueCache<T> {
   pub(crate) fn new(
      jvm_state: GlobalRef,
      jvm: JavaVM,
      update_method_id: JMethodID,
      initial_value: T
   ) -> Self {
      UniqueCache {
         jvm_state,
         jvm,
         update_method_id,
         value: initial_value
      }
   }

   pub fn save(&mut self, value: T) {
      let mut env = self.jvm.get_env().unwrap();
      let java_value = value.clone_into_java(&mut env);

      unsafe {
         env.call_method_unchecked(
            &self.jvm_state,
            self.update_method_id,
            ReturnType::Primitive(Primitive::Void),
            &[JValueGen::Object(java_value).as_jni()]
         ).unwrap();
      }

      self.value = value;
   }
}

#[cfg(not(feature="jvm"))]
impl<T> UniqueCache<T> {
   pub(crate) fn new(initial_state: T) -> Self {
      UniqueCache {
         value: initial_state
      }
   }

   pub fn save(&mut self, value: T) {
      self.value = value;
   }
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> Deref for UniqueCache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
   }
}

#[cfg(not(feature="jvm"))]
impl<T> Deref for UniqueCache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::JObject;

   use super::UniqueCache;

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_UniqueCacheTest_deref(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let initial_value = 42;
      let java_initial_value = env.call_static_method(
         "java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", &[initial_value.into()]
      ).unwrap();
      let jvm_state = env.new_object(
         "com/wcaokaze/probosqis/panoptiqon/UniqueCache", "(Ljava/lang/Object;)V",
         &[java_initial_value.borrow()]
      ).unwrap();
      let jvm_state = env.new_global_ref(jvm_state).unwrap();
      let jvm = env.get_java_vm().unwrap();
      let update_method_id = env.get_method_id(
         "com/wcaokaze/probosqis/panoptiqon/UniqueCache",
         "updateStateFromRust", "(Ljava/lang/Object;)V"
      ).unwrap();
      let mut cache = UniqueCache::new(jvm_state, jvm, update_method_id, 42);
      assert_eq!(42, cache.value);

      assert_eq!(42, *cache);

      cache.save(43);
      assert_eq!(43, cache.value);
   }
}
