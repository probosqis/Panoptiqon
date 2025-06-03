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

use std::sync::{Arc, RwLock};
use serde::Serialize;
use crate::cache::CacheContent;
use crate::db::scheduler::DbScheduler;

#[cfg(feature = "jvm")]
use {
   jni::{JavaVM, JNIEnv},
   jni::objects::{GlobalRef, JMethodID, JObject},
   jni::sys::jlong,
   crate::convert_jvm::{CloneFromJvm, CloneIntoJvm, CloneIntoJvmHelper},
};

#[cfg(feature = "jvm")]
pub(crate) struct JvmUniqueCacheRefs {
   pub class: GlobalRef,
   pub constructor_id: JMethodID,
   pub update_method_id: JMethodID,
   pub cache_class: GlobalRef,
   pub cache_constructor_id: JMethodID,
}

#[cfg(feature = "jvm")]
pub struct UniqueCache<T: CacheContent> {
   jvm: JavaVM,
   jvm_refs: Arc<JvmUniqueCacheRefs>,
   // XXX: この構造体でJVM側のUniqueCacheの強参照を持ち、JVM側のUniqueCacheに
   // この構造体のポインタを渡す際にArcの強参照をインクリメントしているため
   // 循環参照しており絶対に解放されない。
   // jvm_stateと同一インスタンスを指す弱参照も別途持っておき、Arcの強参照が
   // 1(JVMからのものだけ)になったとき強参照のjvm_stateは解放するなどの対応が必要か
   jvm_state: GlobalRef,
   db_scheduler: &'static DbScheduler,
   dir_name: &'static str,
   value: RwLock<Arc<T>>
}

#[cfg(not(feature = "jvm"))]
pub struct UniqueCache<T: CacheContent> {
   db_scheduler: &'static DbScheduler,
   dir_name: &'static str,
   value: RwLock<Arc<T>>
}

#[cfg(feature = "jvm")]
impl<T: CacheContent> UniqueCache<T> {
   pub(crate) fn new_saved<'local>(
      jvm: &'local JavaVM,
      jvm_refs: Arc<JvmUniqueCacheRefs>,
      db_scheduler: &'static DbScheduler,
      dir_name: &'static str,
      initial_value: T
   ) -> Arc<Self>
      where T: CloneIntoJvm<'local, T::JvmType<'local>>
               + CloneIntoJvmHelper
               + Serialize
               + Send + Sync
               + 'static
   {
      use crate::db::save_task::SaveTask;

      let arc = Self::new_arc(jvm, jvm_refs, db_scheduler, dir_name, initial_value);

      let task_cache = Arc::clone(&arc);
      db_scheduler.push(SaveTask::new(dir_name, task_cache));

      arc
   }

   fn new_arc<'local>(
      jvm: &'local JavaVM,
      jvm_refs: Arc<JvmUniqueCacheRefs>,
      db_scheduler: &'static DbScheduler,
      dir_name: &'static str,
      initial_value: T
   ) -> Arc<Self>
      where T: CloneIntoJvm<'local, T::JvmType<'local>> + CloneIntoJvmHelper
   {
      use std::mem::{self, MaybeUninit};

      let mut arc = Arc::new(MaybeUninit::uninit());

      let jvm_state = Self::create_jvm_state(
         jvm, jvm_refs.as_ref(), &initial_value,
         unsafe { mem::transmute(Arc::clone(&arc)) }
      );

      let jvm = unsafe { JavaVM::from_raw(jvm.get_java_vm_pointer()).unwrap() };

      let unique_cache = UniqueCache {
         jvm,
         jvm_refs,
         jvm_state,
         db_scheduler,
         dir_name,
         value: RwLock::new(Arc::new(initial_value))
      };

      unsafe {
         Arc::get_mut_unchecked(&mut arc).write(unique_cache);

         mem::transmute(arc)
      }
   }

   pub(crate) fn get(&self) -> Arc<T> {
      let read_lock = self.value.read().unwrap();
      Arc::clone(&*read_lock)
   }

   pub(crate) fn save<'local>(self: &'local Arc<Self>, value: T)
      where T: CloneIntoJvm<'local, T::JvmType<'local>>
               + Serialize
               + Send + Sync
               + 'static
   {
      use jni::objects::JValueGen;
      use jni::signature::{Primitive, ReturnType};
      use crate::db::save_task::SaveTask;
      use crate::jvm_type::JvmType;

      let mut write_lock = self.value.write().unwrap();

      let mut env = self.jvm.get_env().unwrap();
      let java_value = value.clone_into_jvm(&mut env);

      unsafe {
         env.call_method_unchecked(
            &self.jvm_state,
            self.jvm_refs.update_method_id,
            ReturnType::Primitive(Primitive::Void),
            &[JValueGen::Object(java_value.j_object()).as_jni()]
         ).unwrap();
      }

      *write_lock = Arc::new(value);

      let task_cache = Arc::clone(self);
      self.db_scheduler.push(SaveTask::new(self.dir_name, task_cache));
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

   fn create_jvm_state<'local>(
      jvm: &'local JavaVM,
      jvm_unique_cache_refs: &JvmUniqueCacheRefs,
      initial_value: &T,
      arc: Arc<UniqueCache<T>>
   ) -> GlobalRef
      where T: CloneIntoJvm<'local, T::JvmType<'local>> + CloneIntoJvmHelper
   {
      use jni::objects::JValue;
      use crate::jvm_type::JvmType;

      let mut env = jvm.get_env().unwrap();

      let java_initial_value = initial_value.clone_into_jvm(&mut env);

      let (unique_cache_address, unique_cache_vtable_address)
         = T::get_unique_cache_address(arc);

      let local_object = unsafe {
         env.new_object_unchecked(
               &jvm_unique_cache_refs.class,
               jvm_unique_cache_refs.constructor_id,
               &[
                  JValue::Object(java_initial_value.j_object()).as_jni(),
                  JValue::Long(unique_cache_address as jlong).as_jni(),
                  JValue::Long(unique_cache_vtable_address as jlong).as_jni()
               ]
            )
            .unwrap()
      };

      env.new_global_ref(local_object).unwrap()
   }
}

#[cfg(not(feature = "jvm"))]
impl<T: CacheContent> UniqueCache<T> {
   pub(crate) fn new_saved(
      db_scheduler: &'static DbScheduler,
      dir_name: &'static str,
      initial_state: T
   ) -> Arc<Self>
      where T: Serialize + Send + Sync + 'static
   {
      use crate::db::save_task::SaveTask;

      let cache = UniqueCache {
         db_scheduler,
         dir_name,
         value: RwLock::new(Arc::new(initial_state))
      };

      let arc = Arc::new(cache);

      let task_cache = Arc::clone(&arc);
      db_scheduler.push(SaveTask::new(dir_name, task_cache));

      arc
   }

   pub(crate) fn get(&self) -> Arc<T> {
      let read_lock = self.value.read().unwrap();
      Arc::clone(&*read_lock)
   }

   pub(crate) fn save(self: &Arc<Self>, value: T)
      where T: Serialize + Send + Sync + 'static
   {
      use crate::db::save_task::SaveTask;

      let mut write_lock = self.value.write().unwrap();
      *write_lock = Arc::new(value);

      let task_cache = Arc::clone(self);
      self.db_scheduler.push(SaveTask::new(self.dir_name, task_cache));
   }
}

#[cfg(feature = "jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_WritableUniqueCache_updateNativeState<'local>(
   mut env: JNIEnv<'local>,
   _obj: JObject<'local>,
   unique_cache_address: jlong,
   unique_cache_vtable_address: jlong,
   value: JObject<'local>
) {
   let dyn_unique_cache = get_dyn_two_way_unique_cache(
      unique_cache_address, unique_cache_vtable_address
   );

   unsafe {
      (&*dyn_unique_cache).update_unique_cache(&mut env, value);
   }
}

#[cfg(feature = "jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_UniqueCache_decrementNativeReferenceCount<'local>(
   _env: JNIEnv<'local>,
   _obj: JObject<'local>,
   unique_cache_address: jlong,
   unique_cache_vtable_address: jlong
) {
   let dyn_unique_cache = get_dyn_one_way_unique_cache(
      unique_cache_address, unique_cache_vtable_address
   );

   unsafe {
      (&*dyn_unique_cache).decrement_arc();
   }
}

#[cfg(feature="jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_WritableUniqueCache_decrementNativeReferenceCount<'local>(
   _env: JNIEnv<'local>,
   _obj: JObject<'local>,
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
fn get_dyn_one_way_unique_cache(
   address: jlong,
   vtable_address: jlong
) -> *const dyn DynOneWayUniqueCache {
   use std::mem;
   use std::ptr;

   unsafe {
      let vtable_address = vtable_address as usize;
      let dyn_metadata = mem::transmute(vtable_address);

      ptr::from_raw_parts(address as *const (), dyn_metadata)
   }
}

#[cfg(feature="jvm")]
fn get_dyn_two_way_unique_cache<'local>(
   address: jlong,
   vtable_address: jlong
) -> *const dyn DynTwoWayUniqueCache<'local> {
   use std::mem;
   use std::ptr;

   unsafe {
      let vtable_address = vtable_address as usize;
      let dyn_metadata = mem::transmute(vtable_address);

      ptr::from_raw_parts(address as *const (), dyn_metadata)
   }
}

#[cfg(feature = "jvm")]
pub(crate) trait DynOneWayUniqueCache {
   /// 実装の都合上&selfを受け取るが、呼び出し後参照先のメモリ領域は
   /// 解放されている可能性がある
   unsafe fn decrement_arc(&self);
}

#[cfg(feature = "jvm")]
pub(crate) trait DynTwoWayUniqueCache<'local> {
   fn update_unique_cache(
      &self,
      env: &mut JNIEnv<'local>,
      value: JObject<'local>
   );

   unsafe fn decrement_arc(&self);
}

#[cfg(feature = "jvm")]
impl<T> DynOneWayUniqueCache for UniqueCache<T>
   where T: CacheContent
{
   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
   }
}

#[cfg(feature = "jvm")]
impl<'local, T> DynTwoWayUniqueCache<'local> for UniqueCache<T>
   where T: CacheContent + CloneFromJvm<'local, T::JvmType<'local>> + 'local
{
   fn update_unique_cache(
      &self,
      env: &mut JNIEnv<'local>,
      value: JObject<'local>
   ) {
      use crate::jvm_type::JvmType;

      let value = unsafe { T::JvmType::from_j_object(value) };
      let value = T::clone_from_jvm(env, &value);

      let mut cache_content_lock = self.value.write().unwrap();
      *cache_content_lock = Arc::new(value);
   }

   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
   }
}
