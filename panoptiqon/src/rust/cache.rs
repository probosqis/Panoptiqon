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

#[cfg(feature="jvm")]
use crate::convert_java::ConvertJava;
use crate::unique_cache::UniqueCache;

#[cfg(feature="jvm")]
pub struct Cache<T: ConvertJava>(Arc<Mutex<UniqueCache<T>>>);

#[cfg(not(feature="jvm"))]
pub struct Cache<T>(Arc<Mutex<UniqueCache<T>>>);

#[cfg(feature="jvm")]
impl<T: ConvertJava> Cache<T> {
   pub(crate) fn new(arc: Arc<Mutex<UniqueCache<T>>>) -> Self {
      Cache(arc)
   }

   pub fn lock(&self) -> LockResult<MutexGuard<'_, UniqueCache<T>>> {
      self.0.lock()
   }
}

#[cfg(not(feature="jvm"))]
impl<T> Cache<T> {
   pub(crate) fn new(arc: Arc<Mutex<UniqueCache<T>>>) -> Self {
      Cache(arc)
   }

   pub fn lock(&self) -> LockResult<MutexGuard<'_, UniqueCache<T>>> {
      self.0.lock()
   }

   #[cfg(any(test, feature="jni-test"))]
   pub fn unique_cache_ptr(&self) -> *const Mutex<UniqueCache<T>> {
      Arc::as_ptr(&self.0)
   }
}
