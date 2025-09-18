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

import kotlin.test.Test
import kotlin.test.assertEquals

private typealias TwoWayConversionData = NativeRepositoryTest.TwoWayConversionData

class RepositoryTest {
   init {
      loadNativeLib()
   }

   @Test
   fun restoreNativeRepositoryBorrow() {
      val repository = `restoreNativeRepositoryBorrow$createRepository`()
      `restoreNativeRepositoryBorrow$assertPtr`(repository)
   }

   private external fun `restoreNativeRepositoryBorrow$createRepository`(): Repository<String, TwoWayConversionData>
   private external fun `restoreNativeRepositoryBorrow$assertPtr`(repository: Repository<String, TwoWayConversionData>)

   @Test
   fun gc_dropNativeRepository() {
      `gc_dropNativeRepository$createRepository`()

      System.gc()
      System.runFinalization()

      `gc_dropNativeRepository$assertDropped`()
   }

   private external fun `gc_dropNativeRepository$createRepository`(): Repository<String, TwoWayConversionData>
   private external fun `gc_dropNativeRepository$assertDropped`()

   @Test
   fun load_viaJvmRepository() {
      val repository = `load_viaJvmRepository$createRepository`()

      assertEquals(
         TwoWayConversionData("A", 42),
         repository.load("A").value
      )
   }

   private external fun `load_viaJvmRepository$createRepository`(): Repository<String, TwoWayConversionData>

   @Test
   fun load_viaJvmRepository_sameCache() {
      val repository = `load_viaJvmRepository_sameCache$createRepository`()
      val cache = repository.load("A")

      `load_viaJvmRepository_sameCache$assertSameCache`(cache)
   }

   private external fun `load_viaJvmRepository_sameCache$createRepository`(): Repository<String, TwoWayConversionData>
   private external fun `load_viaJvmRepository_sameCache$assertSameCache`(cache: WritableCache<TwoWayConversionData>)

   @Test
   fun save_viaJvmRepository() {
      val repository = `save_viaJvmRepository$createRepository`()
      repository.save(
         TwoWayConversionData("A", 42)
      )
      `save_viaJvmRepository$assert`()
   }

   private external fun `save_viaJvmRepository$createRepository`(): Repository<String, TwoWayConversionData>
   private external fun `save_viaJvmRepository$assert`(): Repository<String, TwoWayConversionData>

   @Test
   fun save_viaJvmRepository_sameCache() {
      val repository = `save_viaJvmRepository_sameCache$createRepository`()
      val cache = repository.save(
         TwoWayConversionData("A", 42)
      )
      `save_viaJvmRepository_sameCache$assertSameCache`(cache)
   }

   private external fun `save_viaJvmRepository_sameCache$createRepository`(): Repository<String, TwoWayConversionData>
   private external fun `save_viaJvmRepository_sameCache$assertSameCache`(cache: WritableCache<TwoWayConversionData>)
}
