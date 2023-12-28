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

import androidx.compose.material.Text
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.test.junit4.createComposeRule
import com.wcaokaze.probosqis.cache.core.WritableCache
import org.junit.Rule
import kotlin.test.Test
import kotlin.test.assertEquals

class CacheStateComposeTest {
   @get:Rule
   val rule = createComposeRule()

   @Test
   fun recomposition_byOriginalCacheWritten() {
      val cache = WritableCache(42)
      var recompositionCount = 0

      rule.setContent {
         val cacheState by cache.asMutableState()
         Text("$cacheState")

         SideEffect {
            recompositionCount++
         }
      }

      rule.runOnIdle {
         assertEquals(1, recompositionCount)
      }

      cache.value++
      rule.runOnIdle {
         assertEquals(2, recompositionCount)
      }
   }

   @Test
   fun recomposition_byCacheStateUpdated() {
      val cache = WritableCache(42)
      var cacheState by cache.asMutableState()
      var recompositionCount = 0

      rule.setContent {
         Text("$cacheState")

         SideEffect {
            recompositionCount++
         }
      }

      rule.runOnIdle {
         assertEquals(1, recompositionCount)
      }

      cacheState++
      rule.runOnIdle {
         assertEquals(2, recompositionCount)
      }
   }

   @Test
   fun updateCacheState_effectsOriginCache() {
      val cache = WritableCache(42)
      var cacheState by cache.asMutableState()

      @Suppress("UNUSED_CHANGED_VALUE")
      cacheState++

      assertEquals(43, cache.value)
   }
}
