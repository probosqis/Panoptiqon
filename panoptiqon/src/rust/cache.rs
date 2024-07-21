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
   jni::objects::GlobalRef
};

#[cfg(feature="jvm")]
pub struct Cache<T: ConvertJava> {
   jvm_state: GlobalRef,
   jvm: JavaVM,
   value: T
}

#[cfg(not(feature="jvm"))]
pub struct Cache<T> {
   value: T
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> Cache<T> {
   pub(crate) fn new(jvm_state: GlobalRef, jvm: JavaVM, initial_value: T) -> Self {
      Cache {
         jvm_state,
         jvm,
         value: initial_value
      }
   }

   pub fn save(&mut self, value: T) {
      let mut env = self.jvm.get_env().unwrap();
      let java_value = value.clone_into_java(&mut env);
      env.call_method(
         &self.jvm_state,
         "updateStateFromRust", "(Ljava/lang/Object;)V",
         &[(&java_value).into()]
      ).unwrap();

      self.value = value;
   }
}

#[cfg(not(feature="jvm"))]
impl<T> Cache<T> {
   pub(crate) fn new(initial_state: T) -> Self {
      Cache {
         value: initial_state
      }
   }

   pub fn save(&mut self, value: T) {
      self.value = value;
   }
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> Deref for Cache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
   }
}

#[cfg(not(feature="jvm"))]
impl<T> Deref for Cache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
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
      let jvm = env.get_java_vm().unwrap();
      let mut cache = Cache::new(jvm_state, jvm, 42);
      assert_eq!(42, cache.value);

      assert_eq!(42, *cache);

      cache.save(43);
      assert_eq!(43, cache.value);
   }
}
