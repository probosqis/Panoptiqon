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
use std::hash::Hash;

use fnv::FnvHashMap;
#[cfg(feature="jvm")]
use jni::objects::GlobalRef;

use crate::cache::Cache;

pub(crate) struct CachePool<K, T> {
   #[cfg(feature="jvm")]
   jvm_state_instantiation: Box<dyn Fn() -> GlobalRef>,
   map: FnvHashMap<K, Cache<T>>
}

impl<K, T> CachePool<K, T>
   where K: Hash + Eq
{
   #[cfg(feature="jvm")]
   pub fn new(jvm_state_instantiation: Box<dyn Fn() -> GlobalRef>) -> Self {
      CachePool {
         jvm_state_instantiation,
         map: FnvHashMap::default()
      }
   }

   #[cfg(not(feature="jvm"))]
   pub fn new() -> Self {
      CachePool {
         map: FnvHashMap::default()
      }
   }

   pub fn get(&mut self, key: K, initial_value: impl Fn() -> T) -> &Cache<T> {
      #[cfg(feature="jvm")]
      {
         self.map.entry(key).or_insert_with(|| {
            let initial_value = initial_value();

            let jvm_state_instantiation = &self.jvm_state_instantiation;
            let jvm_state = jvm_state_instantiation();
            Cache::new(jvm_state, initial_value)
         })
      }

      #[cfg(not(feature="jvm"))]
      {
         self.map.entry(key).or_insert_with(|| {
            let initial_value = initial_value();
            Cache::new(initial_value)
         })
      }
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::{GlobalRef, JObject};

   use crate::testutils::get_vm;

   use super::CachePool;

   fn instantiate_state_wrapper() -> GlobalRef {
      let vm = get_vm();
      let mut env = vm.get_env().unwrap();
      let local_object = env.new_object("java/lang/Object", "()V", &[]).unwrap();
      env.new_global_ref(local_object).unwrap()
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_createCache(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let state_wrapper_instantiation = Box::new(instantiate_state_wrapper);
      let mut pool = CachePool::new(state_wrapper_instantiation);

      let cache = pool.get("A".to_string(), || 42);
      assert_eq!(42, **cache);

      let cache = pool.get("B".to_string(), || 43);
      assert_eq!(43, **cache);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_pooling(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let state_wrapper_instantiation = Box::new(instantiate_state_wrapper);
      let mut pool = CachePool::new(state_wrapper_instantiation);
      let cache1_ptr = pool.get("A".to_string(), || 42) as *const _;
      let cache2_ptr = pool.get("A".to_string(), || 42) as *const _;
      let cache3_ptr = pool.get("B".to_string(), || 42) as *const _;

      assert_eq!(cache1_ptr, cache2_ptr);
      assert_ne!(cache1_ptr, cache3_ptr);
   }
}
