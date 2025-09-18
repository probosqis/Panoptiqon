/*
 * Copyright 2025 wcaokaze
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

package com.wcaokaze.probosqis.panoptiqon

class Repository<K, T>(
   @get:JvmName("getNativeRepositoryAddress")
   internal val nativeRepositoryAddress: Long,
   private val vtableAddress: Long
) : Object() {
   private external fun dropNativeRepository(
      nativeRepositoryAddress: Long,
      vtableAddress: Long
   )

   fun load(key: K): WritableCache<T> {
      return load(key, nativeRepositoryAddress, vtableAddress)
   }

   private external fun load(
      key: K,
      nativeRepositoryAddress: Long,
      vtableAddress: Long
   ): WritableCache<T>

   fun save(value: T): WritableCache<T> {
      return save(value, nativeRepositoryAddress, vtableAddress)
   }

   private external fun save(
      value: T,
      nativeRepositoryAddress: Long,
      vtableAddress: Long
   ): WritableCache<T>

   @Deprecated("")
   override fun finalize() {
      dropNativeRepository(nativeRepositoryAddress, vtableAddress)
   }
}
