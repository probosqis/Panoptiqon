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

private typealias TwoWayConversionData = NativeRepositoryTest.TwoWayConversionData

class RepositoryTest {
   @Test
   fun restoreNativeRepositoryBorrow() {
      val repository = `restoreNativeRepositoryBorrow$createRepository`()
      `restoreNativeRepositoryBorrow$assertPtr`(repository)
   }

   private external fun `restoreNativeRepositoryBorrow$createRepository`(): Repository<TwoWayConversionData>
   private external fun `restoreNativeRepositoryBorrow$assertPtr`(repository: Repository<TwoWayConversionData>)

   @Test
   fun gc_dropNativeRepository() {
      `gc_dropNativeRepository$createRepository`()

      System.gc()
      System.runFinalization()

      `gc_dropNativeRepository$assertDropped`()
   }

   private external fun `gc_dropNativeRepository$createRepository`(): Repository<TwoWayConversionData>
   private external fun `gc_dropNativeRepository$assertDropped`()
}
