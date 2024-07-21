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
use {
   crate::convert_java::ConvertJava,
   jni::JavaVM,
   jni::JNIEnv,
   jni::objects::GlobalRef
};

use crate::cache::Cache;

#[cfg(feature="jvm")]
pub(crate) struct CachePool<K, T: ConvertJava> {
   jvm: JavaVM,
   map: FnvHashMap<K, Cache<T>>
}

#[cfg(not(feature="jvm"))]
pub(crate) struct CachePool<K, T> {
   map: FnvHashMap<K, Cache<T>>
}

#[cfg(feature="jvm")]
impl<K, T> CachePool<K, T>
   where K: Hash + Eq,
         T: ConvertJava
{
   pub fn new(jvm: JavaVM) -> Self {
      CachePool {
         jvm,
         map: FnvHashMap::default()
      }
   }

   pub fn get(&mut self, key: K, initial_value: impl Fn() -> T) -> &Cache<T> {
      self.map.entry(key).or_insert_with(|| {
         let initial_value = initial_value();

         let mut env = self.jvm.get_env().unwrap();
         let jvm_state = Self::create_jvm_state(&mut env, &initial_value);
         let jvm = unsafe { JavaVM::from_raw(self.jvm.get_java_vm_pointer()).unwrap() };

         Cache::new(jvm_state, jvm, initial_value)
      })
   }

   fn create_jvm_state(env: &mut JNIEnv, initial_value: &T) -> GlobalRef {
      let java_initial_value = initial_value.clone_into_java(env);

      let local_object = env
         .new_object(
            "com/wcaokaze/probosqis/panoptiqon/CacheInternal",
            "(java/lang/Object)V",
            &[(&java_initial_value).into()]
         )
         .unwrap();

      env.new_global_ref(local_object).unwrap()
   }
}

#[cfg(not(feature="jvm"))]
impl<K, T> CachePool<K, T>
   where K: Hash + Eq
{
   pub fn new() -> Self {
      CachePool {
         map: FnvHashMap::default()
      }
   }

   pub fn get(&mut self, key: K, initial_value: impl Fn() -> T) -> &Cache<T> {
      self.map.entry(key).or_insert_with(|| {
         let initial_value = initial_value();
         Cache::new(initial_value)
      })
   }
}

#[cfg(feature="jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::JObject;

   use super::CachePool;

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_createCache(
      env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = CachePool::new(env.get_java_vm().unwrap());

      let cache = pool.get("A".to_string(), || 42);
      assert_eq!(42, **cache);

      let cache = pool.get("B".to_string(), || 43);
      assert_eq!(43, **cache);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CachePoolTest_pooling(
      env: JNIEnv,
      _obj: JObject
   ) {
      let mut pool = CachePool::new(env.get_java_vm().unwrap());
      let cache1_ptr = pool.get("A".to_string(), || 42) as *const _;
      let cache2_ptr = pool.get("A".to_string(), || 42) as *const _;
      let cache3_ptr = pool.get("B".to_string(), || 42) as *const _;

      assert_eq!(cache1_ptr, cache2_ptr);
      assert_ne!(cache1_ptr, cache3_ptr);
   }
}
