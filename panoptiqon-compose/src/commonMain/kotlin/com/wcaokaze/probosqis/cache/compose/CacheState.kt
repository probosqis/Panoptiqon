/*
 * Copyright 2023 wcaokaze
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

package com.wcaokaze.probosqis.cache.compose

import androidx.compose.runtime.Stable
import com.wcaokaze.probosqis.cache.core.Cache
import com.wcaokaze.probosqis.cache.core.WritableCache
import kotlin.reflect.KProperty

fun <T> Cache<T>.asState() = CacheState(this)
fun <T> WritableCache<T>.asMutableState() = WritableCacheState(this)
fun <T> WritableCache<T>.asState(): CacheState<T> = asCache().asState()

@Stable
class CacheState<out T>
   internal constructor(private val cache: Cache<T>)
{
   val value: T by cache::value

   operator fun getValue(thisObj: Any?, property: KProperty<*>): T {
      return cache.value
   }
}

@Stable
class WritableCacheState<T>
   internal constructor(private val cache: WritableCache<T>)
{
   var value: T by cache::value

   operator fun getValue(thisObj: Any?, property: KProperty<*>): T {
      return cache.value
   }

   operator fun setValue(thisObj: Any?, property: KProperty<*>, value: T) {
      cache.value = value
   }
}
