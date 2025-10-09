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

import java.nio.charset.Charset
import kotlin.test.Test
import kotlin.test.assertEquals

class PanoptiqonTest {
   data class CacheContentA(val key: Int, val value: Int)
   data class CacheContentB(val key: Int, val value: String)

   init {
      loadNativeLib()
   }

   @Suppress("TestFunctionName")
   private fun CacheId(repositoryDirPath: String, filePath: String) = Cache.Id(
      repositoryDirPath.toByteArray(Charset.defaultCharset()),
      filePath         .toByteArray(Charset.defaultCharset()),
   )

   @Test
   fun loadJvm() {
      `loadJvm$prepareRepository`()

      val cacheA = `loadJvm$load`(
         CacheId(
            "test/PanoptiqonTest/loadJvm_a",
            "test/PanoptiqonTest/loadJvm_a/0",
         )
      )
      assertEquals(
         CacheContentA(0, 42),
         cacheA.value
      )

      val cacheB = `loadJvm$load`(
         CacheId(
            "test/PanoptiqonTest/loadJvm_b",
            "test/PanoptiqonTest/loadJvm_b/0",
         )
      )
      assertEquals(
         CacheContentB(0, "Lorem ipsum"),
         cacheB.value
      )
   }

   private external fun `loadJvm$prepareRepository`()
   private external fun `loadJvm$load`(id: Cache.Id): Cache<*>
}
