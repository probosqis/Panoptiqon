/*
 * Copyright 2024-2025 wcaokaze
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
   jni::{JavaVM, JNIEnv},
   jni::objects::{GlobalRef, JMethodID, JObject},
   jni::sys::jlong,
   std::mem,
   std::ptr,
   std::sync::{Arc, RwLock},
   crate::convert_java::ConvertJava,
};

#[cfg(feature="jvm")]
pub(crate) struct JvmUniqueCacheRefs {
   pub class: GlobalRef,
   pub constructor_id: JMethodID,
   pub update_method_id: JMethodID,
   pub cache_class: GlobalRef,
   pub cache_constructor_id: JMethodID,
}

#[cfg(feature="jvm")]
type VTable = ();

#[cfg(feature="jvm")]
pub(crate) trait UniqueCacheJniHelper
   where Self: Sized
{
   fn get_jvm_refs(env: &mut JNIEnv) -> JvmUniqueCacheRefs;

   fn get_unique_cache_address(
      unique_cache: Arc<RwLock<UniqueCache<Self>>>
   ) -> (*const RwLock<UniqueCache<Self>>, *const VTable);
}

#[cfg(feature="jvm")]
impl<T> UniqueCacheJniHelper for T where T: ConvertJava {
   fn get_jvm_refs(env: &mut JNIEnv) -> JvmUniqueCacheRefs {
      let class = env
         .find_class("com/wcaokaze/probosqis/panoptiqon/WritableUniqueCache").unwrap();
      let class = env.new_global_ref(class).unwrap();
      let constructor_id = env
         .get_method_id(&class, "<init>", "(Ljava/lang/Object;JJ)V").unwrap();
      let update_method_id = env
         .get_method_id(&class, "updateStateFromNative", "(Ljava/lang/Object;)V").unwrap();

      let cache_class = env
         .find_class("com/wcaokaze/probosqis/panoptiqon/WritableRepositoryCache").unwrap();
      let cache_class = env.new_global_ref(cache_class).unwrap();
      let cache_constructor_id = env
         .get_method_id(&cache_class, "<init>", "(Lcom/wcaokaze/probosqis/panoptiqon/WritableUniqueCache;)V").unwrap();

      JvmUniqueCacheRefs {
         class, constructor_id, update_method_id, cache_class, cache_constructor_id
      }
   }

   fn get_unique_cache_address(
      unique_cache: Arc<RwLock<UniqueCache<Self>>>
   ) -> (*const RwLock<UniqueCache<Self>>, *const VTable) {
      let trait_obj: *const dyn DynTwoWayUniqueCache = Arc::into_raw(unique_cache);
      let unique_cache_address = trait_obj as *const RwLock<UniqueCache<Self>>;

      let trait_object_metadata = ptr::metadata(trait_obj);
      let vtable_address = unsafe {
         mem::transmute::<_, *const ()>(trait_object_metadata)
      };

      (unique_cache_address, vtable_address)
   }
}

#[cfg(feature="jvm")]
pub struct UniqueCache<T> {
   jvm: JavaVM,
   jvm_refs: Arc<JvmUniqueCacheRefs>,
   // XXX: この構造体でJVM側のUniqueCacheの強参照を持ち、JVM側のUniqueCacheに
   // この構造体のポインタを渡す際にArcの強参照をインクリメントしているため
   // 循環参照しており絶対に解放されない。
   // jvm_stateと同一インスタンスを指す弱参照も別途持っておき、Arcの強参照が
   // 1(JVMからのものだけ)になったとき強参照のjvm_stateは解放するなどの対応が必要か
   jvm_state: GlobalRef,
   value: T
}

#[cfg(not(feature="jvm"))]
pub struct UniqueCache<T> {
   value: T
}

#[cfg(feature="jvm")]
impl<T> UniqueCache<T> {
   pub(crate) fn new_arc(
      jvm: &JavaVM,
      jvm_refs: Arc<JvmUniqueCacheRefs>,
      initial_value: T
   ) -> Arc<RwLock<Self>>
      where T: ConvertJava + UniqueCacheJniHelper
   {
      use std::mem::MaybeUninit;

      let arc = Arc::new(RwLock::new(MaybeUninit::uninit()));

      let jvm_state = Self::create_jvm_state(
         jvm, jvm_refs.as_ref(), &initial_value,
         unsafe { mem::transmute(arc.clone()) }
      );

      let jvm = unsafe { JavaVM::from_raw(jvm.get_java_vm_pointer()).unwrap() };

      let unique_cache = UniqueCache {
         jvm,
         jvm_refs,
         jvm_state,
         value: initial_value
      };

      arc.write().unwrap().write(unique_cache);

      unsafe { mem::transmute(arc) }
   }

   pub fn get(&self) -> &T {
      &self.value
   }

   pub fn save(&mut self, value: T)
      where T: ConvertJava
   {
      use jni::objects::JValueGen;
      use jni::signature::{Primitive, ReturnType};

      let mut env = self.jvm.get_env().unwrap();
      let java_value = value.clone_into_java(&mut env);

      unsafe {
         env.call_method_unchecked(
            &self.jvm_state,
            self.jvm_refs.update_method_id,
            ReturnType::Primitive(Primitive::Void),
            &[JValueGen::Object(java_value).as_jni()]
         ).unwrap();
      }

      self.value = value;
   }

   pub(crate) fn create_jvm_cache<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      use jni::objects::JValueGen;

      unsafe {
         env.new_object_unchecked(
            &self.jvm_refs.cache_class,
            self.jvm_refs.cache_constructor_id,
            &[
               JValueGen::Object(&self.jvm_state).as_jni()
            ]
         ).unwrap()
      }
   }

   fn create_jvm_state(
      jvm: &JavaVM,
      jvm_unique_cache_refs: &JvmUniqueCacheRefs,
      initial_value: &T,
      arc: Arc<RwLock<UniqueCache<T>>>
   ) -> GlobalRef
      where T: ConvertJava + UniqueCacheJniHelper
   {
      use jni::objects::JValue;

      let mut env = jvm.get_env().unwrap();

      let java_initial_value = initial_value.clone_into_java(&mut env);

      let (unique_cache_address, unique_cache_vtable_address)
         = T::get_unique_cache_address(arc);

      let local_object = unsafe {
         env.new_object_unchecked(
               &jvm_unique_cache_refs.class,
               jvm_unique_cache_refs.constructor_id,
               &[
                  JValue::Object(&java_initial_value).as_jni(),
                  JValue::Long(unique_cache_address as jlong).as_jni(),
                  JValue::Long(unique_cache_vtable_address as jlong).as_jni()
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

   pub fn get(&self) -> &T {
      &self.value
   }

   pub fn save(&mut self, value: T) {
      self.value = value;
   }
}

impl<T> Deref for UniqueCache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
   }
}

#[cfg(feature="jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_WritableUniqueCache_updateNativeState(
   mut env: JNIEnv,
   _obj: JObject,
   unique_cache_address: jlong,
   unique_cache_vtable_address: jlong,
   value: JObject
) {
   let dyn_unique_cache = get_dyn_two_way_unique_cache(
      unique_cache_address, unique_cache_vtable_address
   );

   unsafe {
      (&*dyn_unique_cache).update_unique_cache(&mut env, &value);
   }
}

#[cfg(feature="jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_WritableUniqueCache_decrementNativeReferenceCount(
   _env: JNIEnv,
   _obj: JObject,
   unique_cache_address: jlong,
   unique_cache_vtable_address: jlong
) {
   let dyn_unique_cache = get_dyn_two_way_unique_cache(
      unique_cache_address, unique_cache_vtable_address
   );

   unsafe {
      (&*dyn_unique_cache).decrement_arc();
   }
}

#[cfg(feature="jvm")]
fn get_dyn_two_way_unique_cache(
   address: jlong,
   vtable_address: jlong
) -> *const dyn DynTwoWayUniqueCache {
   unsafe {
      let vtable_address = vtable_address as usize;
      let dyn_metadata = mem::transmute(vtable_address);

      ptr::from_raw_parts(address as *const (), dyn_metadata)
   }
}

#[cfg(feature="jvm")]
trait DynTwoWayUniqueCache {
   fn update_unique_cache(&self, env: &mut JNIEnv, value: &JObject);

   /// 実装の都合上&selfを受け取るが、呼び出し後参照先のメモリ領域は
   /// 解放されている可能性がある
   unsafe fn decrement_arc(&self);
}

#[cfg(feature="jvm")]
impl<T: ConvertJava> DynTwoWayUniqueCache for RwLock<UniqueCache<T>> {
   fn update_unique_cache(&self, env: &mut JNIEnv, value: &JObject) {
      let value = T::clone_from_java(env, value);
      self.write().unwrap().value = value;
   }

   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
   }
}
