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

import androidx.compose.runtime.mutableStateOf

@Suppress("PLATFORM_CLASS_MAPPED_TO_KOTLIN")
internal class UniqueCache<T>(
   initialValue: T,
   internal val nativeStateAddress: Long,
   private val nativeStateVTableAddress: Long,
) : Object() {
   internal var state = mutableStateOf(initialValue)

   val value: T
      get() = state.value
   
   val id: Cache.Id
      get() = getCacheId(nativeStateAddress, nativeStateVTableAddress)

   // XXX: ネイティブ側のMutexとJVM側のStateはそれぞれスレッドセーフであるが
   // ネイティブ側とJVM側が同時にアクセスされた場合には不整合が起こる可能性がある
   private fun updateStateFromNative(value: T) {
      state.value = value
   }

   private external fun decrementNativeReferenceCount(
      rustStateAddress: Long,
      rustStateVTableAddress: Long
   )
   
   private external fun getCacheId(
      rustStateAddress: Long,
      rustStateVTableAddress: Long
   ): Cache.Id

   @Deprecated("")
   override fun finalize() {
      decrementNativeReferenceCount(nativeStateAddress, nativeStateVTableAddress)
   }
}

@Suppress("PLATFORM_CLASS_MAPPED_TO_KOTLIN")
internal class WritableUniqueCache<T>(
   initialValue: T,
   internal val nativeStateAddress: Long,
   private val nativeStateVTableAddress: Long,
) : Object() {
   internal var state = mutableStateOf(initialValue)

   var value: T
      get() = state.value
      set(value) {
         state.value = value
         updateNativeState(nativeStateAddress, nativeStateVTableAddress, value)
      }

   val id: Cache.Id
      get() = getCacheId(nativeStateAddress, nativeStateVTableAddress)

   // XXX: ネイティブ側のMutexとJVM側のStateはそれぞれスレッドセーフであるが
   // ネイティブ側とJVM側が同時にアクセスされた場合には不整合が起こる可能性がある
   private fun updateStateFromNative(value: T) {
      state.value = value
   }

   private external fun updateNativeState(
      rustStateAddress: Long,
      rustStateVTableAddress: Long,
      value: T
   )

   private external fun decrementNativeReferenceCount(
      rustStateAddress: Long,
      rustStateVTableAddress: Long
   )

   private external fun getCacheId(
      rustStateAddress: Long,
      rustStateVTableAddress: Long
   ): Cache.Id

   @Deprecated("")
   override fun finalize() {
      decrementNativeReferenceCount(nativeStateAddress, nativeStateVTableAddress)
   }
}
