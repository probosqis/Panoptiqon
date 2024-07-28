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

#[cfg(feature="jvm")]
use {
   crate::convert_java::ConvertJava,
   jni::JNIEnv,
};

use crate::pool::UniqueCachePool;

#[cfg(feature="jvm")]
pub struct Repository<K, T: ConvertJava> {
   pool: UniqueCachePool<K, T>
}

#[cfg(not(feature="jvm"))]
pub struct Repository<K, T> {
   pool: UniqueCachePool<K, T>
}

#[cfg(feature="jvm")]
impl<K, T> Repository<K, T>
   where K: Hash + Eq,
         T: ConvertJava
{
   pub fn new(env: &mut JNIEnv) -> Self {
      Repository {
         pool: UniqueCachePool::new(env)
      }
   }
}

#[cfg(not(feature="jvm"))]
impl<K, T> Repository<K, T>
   where K: Hash + Eq
{
   pub fn new() -> Self {
      Repository {
         pool: UniqueCachePool::new()
      }
   }
}
