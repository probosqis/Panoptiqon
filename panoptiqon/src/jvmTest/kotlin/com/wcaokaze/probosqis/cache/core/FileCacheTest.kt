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

package com.wcaokaze.probosqis.cache.core

import kotlinx.serialization.json.Json
import org.junit.After
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.JUnit4
import java.io.File
import java.io.IOException
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue
import kotlin.test.fail

@OptIn(TemporaryCacheApi::class)
@RunWith(JUnit4::class)
class FileCacheTest {
   private val file = File(".fileCacheTest")

   @Before
   fun prepareFile() {
      if (file.exists()) {
         if (!file.deleteRecursively()) { throw IOException() }
      }
   }

   @After
   fun deleteFile() {
      file.deleteRecursively()
   }

   @Test
   fun saveLoad() {
      saveCache(42, file, Json)
      val cache = loadCache<Int>(file, Json)
      assertEquals(42, cache.value)
   }

   @Test
   fun value_writeIntoFile() {
      val cache = saveCache(42, file, Json)
      cache.value++

      val loadedCache = loadCache<Int>(file, Json)
      assertEquals(43, loadedCache.value)
   }

   @Test
   fun load_notFound() {
      assertFailsWith<IOException> {
         loadCache<Int>(file, Json)
      }
   }

   @Test
   fun loadOrDefault() {
      val cache = loadCacheOrDefault(file, Json) { 42 }
      assertEquals(42, cache.value)
      assertTrue(file.exists())
   }

   @Test
   fun loadOrDefault_functionNotCalledWhenLoadSucceed() {
      saveCache(42, file, Json)
      val cache = loadCacheOrDefault<Int>(file, Json) { fail() }
      assertEquals(42, cache.value)
   }

   @Test
   fun save_parentDirNotExists() {
      val dir = file
      dir.deleteRecursively()
      saveCache(42, File(dir, "file"), Json)
   }
}
