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
use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::{Deserialize, Deserializer};
use crate::unique_cache::UniqueCache;

#[cfg(feature="jvm")]
use {
   jni::JNIEnv,
   jni::objects::JObject,
   crate::convert_jni::CloneIntoJni,
   crate::jvm_type::JvmType,
};

#[derive(Clone)]
pub struct Cache<T: CacheContent>(Arc<RwLock<UniqueCache<T>>>);

impl<T: CacheContent> Cache<T> {
   pub(crate) fn new(arc: Arc<RwLock<UniqueCache<T>>>) -> Self {
      Cache(arc)
   }

   pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, UniqueCache<T>>> {
      self.0.write()
   }

   pub fn read(&self) -> LockResult<RwLockReadGuard<'_, UniqueCache<T>>> {
      self.0.read()
   }

   #[cfg(feature="jvm")]
   pub fn create_jvm_instance<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local>
      where T: CloneIntoJni
   {
      let unique_cache_lock = self.0.read().unwrap();
      unique_cache_lock.create_jvm_cache(env)
   }

   /// RepositoryCacheのインスタンスからCacheを生成する。
   ///
   /// # Safety
   /// 指定したJVMインスタンスに対応するネイティブ側の[UniqueCache]のメモリ領域が
   /// Tと違う型を格納している場合、この関数の返り値のCacheに対するすべての動作は
   /// 未定義となる。
   #[cfg(feature="jvm")]
   pub unsafe fn from_jvm_instance<'local>(
      env: &mut JNIEnv<'local>,
      java_instance: &JObject
   ) -> Self {
      let address = env.call_method(
         &java_instance, "getUniqueCacheRustStateAddress", "()J", &[]
      ).unwrap().j().unwrap() as *const _;

      let unique_cache = unsafe { Arc::<RwLock<_>>::from_raw(address) };
      Cache::new(unique_cache.clone())
   }

   #[cfg(any(test, feature="jni-test"))]
   pub fn unique_cache_ptr(&self) -> *const RwLock<UniqueCache<T>> {
      Arc::as_ptr(&self.0)
   }
}

impl<T: CacheContent> Debug for Cache<T> where T: Debug {
   fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      let cache_lock = self.read().unwrap();
      let value = cache_lock.get();
      write!(f, "Cache({:?})", value)
   }
}

impl<T: CacheContent> PartialEq for Cache<T> where T: PartialEq {
   fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
   }
}

impl<T: CacheContent> Eq for Cache<T> where T: Eq {}

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
