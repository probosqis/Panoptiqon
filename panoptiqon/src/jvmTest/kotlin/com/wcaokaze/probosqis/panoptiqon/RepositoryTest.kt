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

import kotlin.test.Test
import kotlin.test.assertEquals

class RepositoryTest {
   init {
      loadNativeLib()
   }

   @Test
   external fun saveLoad()

   @Test
   external fun load_noSuchCache()

   @Test
   external fun save_viaCache()

   @Test
   external fun save_affectAnotherCache()

   @Test
   fun jvmCache() {
      val cache = `jvmCache$getCache`()
      assertEquals(42, cache.value)
   }

   external fun `jvmCache$getCache`(): Cache<Int>

   @Test
   fun jvmCache_valueChangeFromNative() {
      val cache = `jvmCache_valueChangeFromNative$getCache`()
      assertEquals(42, cache.value)
      `jvmCache_valueChangeFromNative$changeValue`()
      assertEquals(13, cache.value)
   }

   external fun `jvmCache_valueChangeFromNative$getCache`(): Cache<Int>
   external fun `jvmCache_valueChangeFromNative$changeValue`()

   @Test
   fun jvmCache_valueChangeFromJvm() {
      val cache = `jvmCache_valueChangeFromJvm$getCache`()
      assertEquals(42, cache.value)
      cache.value = 13
      `jvmCache_valueChangeFromJvm$assertValue`()
   }

   external fun `jvmCache_valueChangeFromJvm$getCache`(): WritableCache<Int>
   external fun `jvmCache_valueChangeFromJvm$assertValue`()

   @Test
   fun jvmCache_valueChange_doesntAffectOtherKeyCaches() {
      `jvmCache_valueChange_doesntAffectOtherKeyCaches$createRepository`()
      val cacheA = `jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheA`()
      val cacheB = `jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheB`()
      val cacheC = `jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheC`()
      assertEquals(0, cacheA.value)
      assertEquals(1, cacheB.value)
      assertEquals(2, cacheC.value)
      `jvmCache_valueChange_doesntAffectOtherKeyCaches$changeCacheB`()
      assertEquals(0, cacheA.value)
      assertEquals(3, cacheB.value)
      assertEquals(2, cacheC.value)

      cacheB.value++
      `jvmCache_valueChange_doesntAffectOtherKeyCaches$assertCacheB`()
   }

   external fun `jvmCache_valueChange_doesntAffectOtherKeyCaches$createRepository`()
   external fun `jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheA`(): WritableCache<Int>
   external fun `jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheB`(): WritableCache<Int>
   external fun `jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheC`(): WritableCache<Int>
   external fun `jvmCache_valueChange_doesntAffectOtherKeyCaches$changeCacheB`()
   external fun `jvmCache_valueChange_doesntAffectOtherKeyCaches$assertCacheB`()
}