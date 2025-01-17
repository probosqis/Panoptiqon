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

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class RepositoryTest {
   init {
      loadNativeLib()
   }

   data class OneWayConversionData(val first: String, val second: Int)

   @Test
   fun switchCacheClass() {
      val oneWayCache = `switchCacheClass$saveOneWayData`()
      val twoWayCache = `switchCacheClass$saveTwoWayData`()

      assertIs<RepositoryCache<*>>(oneWayCache)
      assertIs<WritableRepositoryCache<*>>(twoWayCache)
   }

   private external fun `switchCacheClass$saveOneWayData`(): Cache<OneWayConversionData>
   private external fun `switchCacheClass$saveTwoWayData`(): WritableCache<Int>

   @Test
   external fun saveLoad()

   @Test
   external fun load_noSuchCache()

   @Test
   external fun save_viaCache()

   @Test
   external fun save_affectAnotherCache()

   @Test
   fun oneWay_jvmCache() {
      val cache = `oneWay_jvmCache$getCache`()
      assertEquals(OneWayConversionData("A", 42), cache.value)
   }

   private external fun `oneWay_jvmCache$getCache`(): Cache<OneWayConversionData>

   @Test
   fun twoWay_jvmCache() {
      val cache = `twoWay_jvmCache$getCache`()
      assertEquals(Pair("A", 42), cache.value)
   }

   private external fun `twoWay_jvmCache$getCache`(): Cache<Pair<String, Int>>

   @Test
   fun oneWay_jvmCache_valueChangeFromNative() {
      val cache = `oneWay_jvmCache_valueChangeFromNative$getCache`()
      assertEquals(OneWayConversionData("A", 42), cache.value)
      `oneWay_jvmCache_valueChangeFromNative$changeValue`()
      assertEquals(OneWayConversionData("A", 13), cache.value)
   }

   private external fun `oneWay_jvmCache_valueChangeFromNative$getCache`(): Cache<OneWayConversionData>
   private external fun `oneWay_jvmCache_valueChangeFromNative$changeValue`()

   @Test
   fun twoWay_jvmCache_valueChangeFromNative() {
      val cache = `twoWay_jvmCache_valueChangeFromNative$getCache`()
      assertEquals(Pair("A", 42), cache.value)
      `twoWay_jvmCache_valueChangeFromNative$changeValue`()
      assertEquals(Pair("A", 13), cache.value)
   }

   private external fun `twoWay_jvmCache_valueChangeFromNative$getCache`(): Cache<Pair<String, Int>>
   private external fun `twoWay_jvmCache_valueChangeFromNative$changeValue`()

   @Test
   fun jvmCache_valueChangeFromJvm() {
      val cache = `jvmCache_valueChangeFromJvm$getCache`()
      assertEquals(Pair("A", 42), cache.value)
      cache.value = Pair("A", 13)
      `jvmCache_valueChangeFromJvm$assertValue`()
   }

   private external fun `jvmCache_valueChangeFromJvm$getCache`(): WritableCache<Pair<String, Int>>
   private external fun `jvmCache_valueChangeFromJvm$assertValue`()

   @Test
   fun oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches() {
      `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$createRepository`()
      val cacheA = `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheA`()
      val cacheB = `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheB`()
      val cacheC = `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheC`()
      assertEquals(OneWayConversionData("A", 0), cacheA.value)
      assertEquals(OneWayConversionData("B", 1), cacheB.value)
      assertEquals(OneWayConversionData("C", 2), cacheC.value)
      `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$changeCacheB`()
      assertEquals(OneWayConversionData("A", 0), cacheA.value)
      assertEquals(OneWayConversionData("B", 3), cacheB.value)
      assertEquals(OneWayConversionData("C", 2), cacheC.value)
   }

   private external fun `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$createRepository`()
   private external fun `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheA`(): Cache<OneWayConversionData>
   private external fun `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheB`(): Cache<OneWayConversionData>
   private external fun `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheC`(): Cache<OneWayConversionData>
   private external fun `oneWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$changeCacheB`()

   @Test
   fun twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches() {
      `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$createRepository`()
      val cacheA = `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheA`()
      val cacheB = `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheB`()
      val cacheC = `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheC`()
      assertEquals(Pair("A", 0), cacheA.value)
      assertEquals(Pair("B", 1), cacheB.value)
      assertEquals(Pair("C", 2), cacheC.value)
      `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$changeCacheB`()
      assertEquals(Pair("A", 0), cacheA.value)
      assertEquals(Pair("B", 3), cacheB.value)
      assertEquals(Pair("C", 2), cacheC.value)

      cacheB.value = Pair("B", 4)
      `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$assertCacheB`()
   }

   private external fun `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$createRepository`()
   private external fun `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheA`(): WritableCache<Pair<String, Int>>
   private external fun `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheB`(): WritableCache<Pair<String, Int>>
   private external fun `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$getCacheC`(): WritableCache<Pair<String, Int>>
   private external fun `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$changeCacheB`()
   private external fun `twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches$assertCacheB`()
}
