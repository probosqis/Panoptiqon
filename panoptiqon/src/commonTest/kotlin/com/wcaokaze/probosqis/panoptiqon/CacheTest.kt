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

import kotlin.test.Ignore
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIsNot
import kotlin.test.assertNotSame

class CacheTest {
   @Test
   fun value() {
      val cache = WritableCache(42)
      cache.value++

      assertEquals(43, cache.value)
   }

   @Test
   fun asCache() {
      val writableCache = WritableCache(42)
      val cache = writableCache.asCache()
      writableCache.value++

      assertEquals(43, cache.value)
   }

   @Ignore
   @Test
   fun asCache_notSameInstance() {
      val writableCache = WritableCache(42)
      val cache = writableCache.asCache()

      assertNotSame(writableCache as Any, cache as Any)
      assertIsNot<WritableCache<*>>(cache)
   }
}
