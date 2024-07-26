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
use std::sync::Mutex;

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   jni::{JavaVM, JNIEnv},
   jni::objects::{GlobalRef, JMethodID, JObject, JValueGen},
   jni::signature::{Primitive, ReturnType},
   jni::sys::jlong,
   std::mem::{self},
   std::ptr::{self},
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

#[cfg(feature="jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_UniqueCache_updateRustState(
   mut env: JNIEnv,
   _obj: JObject,
   updater_address: jlong,
   updater_vtable_address: jlong,
   value: JObject
) {
   unsafe {
      let dyn_metadata = {
         let updater_vtable_address = updater_vtable_address as usize;
         mem::transmute(updater_vtable_address)
      };

      let updater_ptr: *const dyn UpdateUniqueCache = ptr::from_raw_parts(
         updater_address as *const (),
         dyn_metadata
      );

      (&*updater_ptr).update_unique_cache(&mut env, value);
   }
}

#[cfg(feature="jvm")]
pub(crate) trait UpdateUniqueCache {
   fn update_unique_cache(&self, env: &mut JNIEnv, value: JObject);
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> UpdateUniqueCache for Mutex<UniqueCache<T>> {
   fn update_unique_cache(&self, env: &mut JNIEnv, value: JObject) {
      let value = T::clone_from_java(env, value);
      self.lock().unwrap().value = value;
   }
}
