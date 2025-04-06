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

use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::sync::Arc;
use serde::{Deserialize, Deserializer};
use crate::unique_cache::UniqueCache;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   jni::objects::JObject,
   crate::convert_jvm::CloneIntoJvm,
   crate::jvm_type::JvmType,
};

pub struct Cache<T: CacheContent>(Arc<UniqueCache<T>>);

impl<T: CacheContent> Cache<T> {
   pub(crate) fn new(arc: Arc<UniqueCache<T>>) -> Self {
      Cache(arc)
   }

   pub fn get(&self) -> Arc<T> {
      self.0.get()
   }

   #[cfg(feature = "jvm")]
   pub fn save(&self, value: T)
      where for<'local> T: CloneIntoJvm<'local, T::JvmType<'local>>
   {
      self.0.save(value);
   }

   #[cfg(not(feature = "jvm"))]
   pub fn save(&self, value: T) {
      self.0.save(value);
   }

   #[cfg(feature = "jvm")]
   pub fn create_jvm_instance<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.0.create_jvm_cache(env)
   }

   /// RepositoryCacheのインスタンスからCacheを生成する。
   ///
   /// # Safety
   /// 指定したJVMインスタンスに対応するネイティブ側の[UniqueCache]のメモリ領域が
   /// Tと違う型を格納している場合、この関数の返り値のCacheに対するすべての動作は
   /// 未定義となる。
   #[cfg(feature = "jvm")]
   pub unsafe fn from_jvm_instance<'local>(
      env: &mut JNIEnv<'local>,
      java_instance: &JObject
   ) -> Self {
      let address = env.call_method(
         &java_instance, "getUniqueCacheRustStateAddress", "()J", &[]
      ).unwrap().j().unwrap() as *const UniqueCache<T>;

      let unique_cache = unsafe { Arc::<_>::from_raw(address) };
      Cache::new(unique_cache.clone())
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub fn unique_cache_ptr(&self) -> *const UniqueCache<T> {
      Arc::as_ptr(&self.0)
   }
}

impl<T: CacheContent> Debug for Cache<T> where T: Debug {
   fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      let value = self.get();
      write!(f, "Cache({:?})", *value)
   }
}

impl<T: CacheContent> PartialEq for Cache<T> {
   fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
   }
}

impl<T: CacheContent> Eq for Cache<T> {}

impl<T: CacheContent> Clone for Cache<T> {
   fn clone(&self) -> Self {
      Cache(self.0.clone())
   }
}

impl<'de, T: CacheContent> Deserialize<'de> for Cache<T> {
   fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
      where D: Deserializer<'de>
   {
      Err(serde::de::Error::custom("not implemented"))
   }
}

pub trait CacheContent {
   type Key: Hash + Eq;

   #[cfg(feature = "jvm")]
   type JvmType<'local>: JvmType<'local>;

   fn key(&self) -> Self::Key;
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use jni::JNIEnv;
   use jni::objects::JObject;
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};
   use crate::jvm_type;
   use super::Cache;

   jvm_type! {
      JvmCacheContentImpl,
   }

   #[derive(Debug, PartialEq, Eq)]
   struct CacheContentImpl(i32);

   impl CacheContent for CacheContentImpl {
      type Key = i32;
      type JvmType<'local> = JvmCacheContentImpl<'local>;

      fn key(&self) -> i32 {
         self.0
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmCacheContentImpl<'local>> for CacheContentImpl {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCacheContentImpl<'local> {
         use crate::jvm_type::JvmType;

         let j_object = env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/CacheTest$CacheContentImpl;",
            "(I)V",
            &[self.0.into()]
         ).unwrap();

         unsafe { JvmCacheContentImpl::from_j_object(j_object) }
      }
   }

   impl<'local> CloneFromJvm<'local, JvmCacheContentImpl<'local>> for CacheContentImpl {
      fn clone_from_jvm(
         env: &mut JNIEnv<'local>,
         jvm_instance: &JvmCacheContentImpl<'local>
      ) -> CacheContentImpl {
         use crate::jvm_type::JvmType;

         let value = env
            .call_method(jvm_instance.j_object(), "getValue", "()I", &[]).unwrap()
            .i().unwrap();

         CacheContentImpl(value)
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1withoutJvmCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new(&mut env);
      let cache = repository.save(CacheContentImpl(42));

      // pool内、JVM、cache
      assert_eq!(3, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1withJvmCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new(&mut env);
      let cache = repository.save(CacheContentImpl(42));

      let _jvm_cache = cache.create_jvm_instance(&mut env);

      // pool内、JVM、cache
      assert_eq!(3, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1clone<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new(&mut env);
      let cache = repository.save(CacheContentImpl(42));

      let _clone = cache.clone();

      // pool内、JVM、cache, _clone
      assert_eq!(4, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1cloneIntoJvm<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::jvm_types::JvmCache;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new(&mut env);
      let cache = repository.save(CacheContentImpl(42));

      let _jvm_cache: JvmCache<JvmCacheContentImpl>
         = cache.clone_into_jvm(&mut env);

      // pool内、JVM、cache
      assert_eq!(3, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1cloneIntoJvm_1cloneFromJvm<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::jvm_types::JvmCache;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new(&mut env);
      let cache = repository.save(CacheContentImpl(42));

      let jvm_cache: JvmCache<JvmCacheContentImpl>
         = cache.clone_into_jvm(&mut env);

      let _clone = Cache::<CacheContentImpl>::clone_from_jvm(&mut env, &jvm_cache);

      // pool内、JVM、cache, _clone
      assert_eq!(4, Arc::strong_count(&cache.0));
   }
}
