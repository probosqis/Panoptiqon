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
use std::sync::{Arc, Mutex};

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   jni::{JavaVM, JNIEnv},
   jni::objects::{GlobalRef, JMethodID, JObject, JValue, JValueGen},
   jni::signature::{Primitive, ReturnType},
   jni::sys::jlong,
   std::mem::{self, MaybeUninit},
   std::ptr::{self},
};

#[cfg(feature="jvm")]
pub(crate) struct JvmUniqueCacheRefs {
   pub class: GlobalRef,
   pub constructor_id: JMethodID,
   pub update_method_id: JMethodID,
}

#[cfg(feature="jvm")]
impl JvmUniqueCacheRefs {
   pub(crate) fn new(env: &mut JNIEnv) -> Self {
      let class = env.find_class("com/wcaokaze/probosqis/panoptiqon/UniqueCache").unwrap();
      let class = env.new_global_ref(class).unwrap();
      let constructor_id = env
         .get_method_id(&class, "<init>", "(Ljava/lang/Object;JJ)V").unwrap();
      let update_method_id = env
         .get_method_id(&class, "updateStateFromRust", "(Ljava/lang/Object;)V").unwrap();

      JvmUniqueCacheRefs { class, constructor_id, update_method_id }
   }
}

#[cfg(feature="jvm")]
pub struct UniqueCache<T: ConvertJava> {
   jvm: JavaVM,
   jvm_state: GlobalRef,
   jvm_state_update_method_id: JMethodID,
   value: T
}

#[cfg(not(feature="jvm"))]
pub struct UniqueCache<T> {
   value: T
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> UniqueCache<T> {
   pub(crate) fn new_arc(
      jvm: &JavaVM,
      jvm_refs: &JvmUniqueCacheRefs,
      initial_value: T
   ) -> Arc<Mutex<Self>> {
      let arc = Arc::new(Mutex::new(MaybeUninit::uninit()));

      let jvm_state = Self::create_jvm_state(
         jvm, jvm_refs, &initial_value,
         unsafe { mem::transmute(arc.clone()) }
      );

      let jvm = unsafe { JavaVM::from_raw(jvm.get_java_vm_pointer()).unwrap() };

      let unique_cache = UniqueCache {
         jvm,
         jvm_state,
         jvm_state_update_method_id: jvm_refs.update_method_id,
         value: initial_value
      };

      arc.lock().unwrap().write(unique_cache);

      unsafe { mem::transmute(arc) }
   }

   pub fn save(&mut self, value: T) {
      let mut env = self.jvm.get_env().unwrap();
      let java_value = value.clone_into_java(&mut env);

      unsafe {
         env.call_method_unchecked(
            &self.jvm_state,
            self.jvm_state_update_method_id,
            ReturnType::Primitive(Primitive::Void),
            &[JValueGen::Object(java_value).as_jni()]
         ).unwrap();
      }

      self.value = value;
   }

   fn create_jvm_state(
      jvm: &JavaVM,
      jvm_unique_cache_refs: &JvmUniqueCacheRefs,
      initial_value: &T,
      arc: Arc<Mutex<UniqueCache<T>>>
   ) -> GlobalRef {
      let mut env = jvm.get_env().unwrap();

      let java_initial_value = initial_value.clone_into_java(&mut env);
      let trait_obj: *const dyn DynUniqueCache = Arc::into_raw(arc);
      let dyn_metadata = ptr::metadata(trait_obj);
      let vtable_ptr = unsafe { mem::transmute::<_, usize>(dyn_metadata) };

      let local_object = unsafe {
         env.new_object_unchecked(
               &jvm_unique_cache_refs.class,
               jvm_unique_cache_refs.constructor_id,
               &[
                  JValue::Object(&java_initial_value).as_jni(),
                  JValue::Long(trait_obj as *const () as jlong).as_jni(),
                  JValue::Long(vtable_ptr as jlong).as_jni()
               ]
            )
            .unwrap()
      };

      env.new_global_ref(local_object).unwrap()
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
   unique_cache_address: jlong,
   unique_cache_vtable_address: jlong,
   value: JObject
) {
   let dyn_unique_cache = get_dyn_unique_cache(
      unique_cache_address, unique_cache_vtable_address
   );

   unsafe {
      (&*dyn_unique_cache).update_unique_cache(&mut env, value);
   }
}

#[cfg(feature="jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_UniqueCache_decrementRustReferenceCount(
   _env: JNIEnv,
   _obj: JObject,
   unique_cache_address: jlong,
   unique_cache_vtable_address: jlong
) {
   let dyn_unique_cache = get_dyn_unique_cache(
      unique_cache_address, unique_cache_vtable_address
   );

   unsafe {
      (&*dyn_unique_cache).decrement_arc();
   }
}

#[cfg(feature="jvm")]
fn get_dyn_unique_cache(
   address: jlong,
   vtable_address: jlong
) -> *const dyn DynUniqueCache {
   unsafe {
      let vtable_address = vtable_address as usize;
      let dyn_metadata = mem::transmute(vtable_address);

      ptr::from_raw_parts(address as *const (), dyn_metadata)
   }
}

#[cfg(feature="jvm")]
trait DynUniqueCache {
   fn update_unique_cache(&self, env: &mut JNIEnv, value: JObject);

   /// 実装の都合上&selfを受け取るが、呼び出し後参照先のメモリ領域は
   /// 解放されている可能性がある
   unsafe fn decrement_arc(&self);
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> DynUniqueCache for Mutex<UniqueCache<T>> {
   fn update_unique_cache(&self, env: &mut JNIEnv, value: JObject) {
      let value = T::clone_from_java(env, value);
      self.lock().unwrap().value = value;
   }

   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
   }
}
