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

package com.wcaokaze.probosqis.panoptiqon

internal class WritableRepositoryCache<T>(
   private val uniqueCache: WritableUniqueCache<T>
) : Cache<T>, WritableCache<T> {
   val uniqueCacheRustStateAddress: Long
      get() = uniqueCache.nativeStateAddress

   @InternalCacheApi
   override val state get() = uniqueCache.state

   @InternalCacheApi
   override val mutableState get() = uniqueCache.state

   override var value: T by uniqueCache::value

   override fun asCache(): Cache<T> = this

   override fun hashCode() = uniqueCache.hashCode()
   override fun equals(other: Any?)
       = other is WritableRepositoryCache<*> && uniqueCache === other.uniqueCache
}
