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

package com.wcaokaze.probosqis.panoptiqon

import androidx.compose.runtime.mutableStateOf

@Suppress("PLATFORM_CLASS_MAPPED_TO_KOTLIN")
internal class UniqueCache<T>(
   initialValue: T,
   private val rustStateAddress: Long,
   private val rustStateVTableAddress: Long,
) : Object() {
   private var state = mutableStateOf(initialValue)

   var value: T
      get() = state.value
      set(value) {
         state.value = value
         updateRustState(rustStateAddress, rustStateVTableAddress, value)
      }

   // XXX: Rust側のMutexとJVM側のStateはそれぞれスレッドセーフであるが
   // Rust側とJVM側が同時にアクセスされた場合には不整合が起こる可能性がある
   fun updateStateFromRust(value: T) {
      state.value = value
   }

   external fun updateRustState(
      rustStateAddress: Long,
      rustStateVTableAddress: Long,
      value: T
   )

   external fun decrementRustReferenceCount(
      rustStateAddress: Long,
      rustStateVTableAddress: Long
   )

   @Deprecated("")
   override fun finalize() {
      decrementRustReferenceCount(rustStateAddress, rustStateVTableAddress)
   }
}
