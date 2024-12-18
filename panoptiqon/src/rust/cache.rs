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
use std::sync::{Arc, LockResult, Mutex, MutexGuard};

use serde::{Deserialize, Deserializer};

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   jni::JNIEnv,
   jni::objects::JObject,
};

use crate::unique_cache::UniqueCache;

pub struct Cache<T>(Arc<Mutex<UniqueCache<T>>>);

impl<T> Cache<T> {
   pub(crate) fn new(arc: Arc<Mutex<UniqueCache<T>>>) -> Self {
      Cache(arc)
   }

   pub fn lock(&self) -> LockResult<MutexGuard<'_, UniqueCache<T>>> {
      self.0.lock()
   }

   #[cfg(feature="jvm")]
   pub fn create_jvm_instance<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local>
      where T: ConvertJava
   {
      let unique_cache_lock = self.0.lock().unwrap();
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
   ) -> Self
      where T: ConvertJava
   {
      let address = env.call_method(
         &java_instance, "getUniqueCacheRustStateAddress", "()J", &[]
      ).unwrap().j().unwrap() as *const _;

      let unique_cache = unsafe { Arc::<Mutex<_>>::from_raw(address) };
      Cache::new(unique_cache.clone())
   }

   #[cfg(any(test, feature="jni-test"))]
   pub fn unique_cache_ptr(&self) -> *const Mutex<UniqueCache<T>> {
      Arc::as_ptr(&self.0)
   }
}

impl<'de, T> Deserialize<'de> for Cache<T> {
   fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
      where D: Deserializer<'de>
   {
      Err(serde::de::Error::custom("not implemented"))
   }
}
