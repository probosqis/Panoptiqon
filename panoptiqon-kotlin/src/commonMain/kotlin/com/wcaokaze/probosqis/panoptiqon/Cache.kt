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

package com.wcaokaze.probosqis.panoptiqon

import androidx.compose.runtime.MutableState
import androidx.compose.runtime.State
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

@RequiresOptIn
annotation class InternalCacheApi

interface Cache<out T> {
   val value: T

   @InternalCacheApi
   val state: State<T>
}

fun <T> Cache(initialValue: T): Cache<T>
      = CacheImpl(initialValue)

interface WritableCache<T> {
   var value: T

   fun asCache(): Cache<T>

   @InternalCacheApi
   val mutableState: MutableState<T>
}

fun <T> WritableCache(initialValue: T): WritableCache<T>
      = CacheImpl(initialValue)

inline fun <T> WritableCache<T>.update(update: (T) -> T) {
   value = update(value)
}

private class CacheImpl<T>(initialValue: T) : Cache<T>, WritableCache<T> {
   private val _state = mutableStateOf(initialValue)

   @InternalCacheApi
   override val state get() = _state

   @InternalCacheApi
   override val mutableState get() = _state

   override var value: T by _state

   override fun asCache(): Cache<T> = this
}
