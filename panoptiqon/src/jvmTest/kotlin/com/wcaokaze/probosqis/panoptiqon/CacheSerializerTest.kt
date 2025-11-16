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

import kotlinx.serialization.Contextual
import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.modules.SerializersModule
import java.io.IOException
import java.nio.charset.Charset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class CacheSerializerTest {
   init {
      loadNativeLib()
   }

   @Serializable
   data class CacheContentImpl(
      val id: Long,
      val content: String
   )

   @Serializable
   data class CacheContainer(
      @Contextual
      val cache: Cache<CacheContentImpl>
   )

   @Serializable
   data class WritableCacheContainer(
      @Contextual
      val cache: WritableCache<CacheContentImpl>
   )

   private fun createJsonSerialization(loadCache: (CacheId) -> Cache<*>) = Json {
      class CacheSerializer<T>(
         @Suppress("UNUSED_PARAMETER")
         contentSerializer: KSerializer<T>
      ) : AbstractCacheSerializer<T>() {
         override fun loadCache(cacheId: CacheId): Cache<T> {
            @Suppress("UNCHECKED_CAST")
            return loadCache(cacheId) as Cache<T>
         }
      }

      serializersModule = SerializersModule {
         contextual(Cache::class) { args -> CacheSerializer(args[0]) }
      }
   }

   @JvmName("createWritableCacheJsonSerialization")
   private fun createJsonSerialization(loadCache: (CacheId) -> WritableCache<*>) = Json {
      class WritableCacheSerializer<T>(
         @Suppress("UNUSED_PARAMETER")
         contentSerializer: KSerializer<T>
      ) : AbstractWritableCacheSerializer<T>() {
         override fun loadCache(cacheId: CacheId): WritableCache<T> {
            @Suppress("UNCHECKED_CAST")
            return loadCache(cacheId) as WritableCache<T>
         }
      }

      serializersModule = SerializersModule {
         contextual(WritableCache::class) { args -> WritableCacheSerializer(args[0]) }
      }
   }

   private fun cacheContentImplJsonStr(
      repositoryDirPath: String,
      filePath: String
   ): String {
      val systemCharset = Charset.defaultCharset()
      val repositoryDirPathBytes  = repositoryDirPath.toByteArray(systemCharset)
      val repositoryFilePathBytes = filePath         .toByteArray(systemCharset)
      val repositoryDirPathJson  = repositoryDirPathBytes .joinToString(separator = ",", prefix = "[", postfix = "]") { it.toString() }
      val repositoryFilePathJson = repositoryFilePathBytes.joinToString(separator = ",", prefix = "[", postfix = "]") { it.toString() }
      return "{\"repositoryDirPath\":$repositoryDirPathJson,\"filePath\":$repositoryFilePathJson}"
   }

   private fun cacheContainerJsonStr(
      repositoryDirPath: String,
      filePath: String
   ): String {
      val cacheContentJson = cacheContentImplJsonStr(repositoryDirPath, filePath)
      return "{\"cache\":$cacheContentJson}"
   }

   @Test
   fun serialize() {
      val json = createJsonSerialization(::`serialize$loadCache`)

      val cache = `serialize$saveCache`()
      val cacheContainer = CacheContainer(cache)

      val jsonStr = json.encodeToString(cacheContainer)
      assertEquals(
         cacheContainerJsonStr(
            "CacheSerializerTest/serialize",
            "CacheSerializerTest/serialize/0"
         ),
         jsonStr
      )
   }

   private external fun `serialize$saveCache`(): Cache<CacheContentImpl>
   private external fun `serialize$loadCache`(cacheId: CacheId): Cache<*>

   @Test
   fun serialize_writable() {
      val json = createJsonSerialization(::`serialize_writable$loadCache`)

      val cache = `serialize_writable$saveCache`()
      val cacheContainer = WritableCacheContainer(cache)

      val jsonStr = json.encodeToString(cacheContainer)
      assertEquals(
         cacheContainerJsonStr(
            "CacheSerializerTest/serialize_writable",
            "CacheSerializerTest/serialize_writable/0"
         ),
         jsonStr
      )
   }

   private external fun `serialize_writable$saveCache`(): WritableCache<CacheContentImpl>
   private external fun `serialize_writable$loadCache`(cacheId: CacheId): WritableCache<*>

   @Test
   fun deserialize() {
      `deserialize$preparePanoptiqon`()

      val json = createJsonSerialization(::`deserialize$loadCache`)

      val cacheContainerJson = cacheContainerJsonStr(
         "CacheSerializerTest/deserialize",
         "CacheSerializerTest/deserialize/0"
      )
      val cache = json.decodeFromString<CacheContainer>(cacheContainerJson)

      assertEquals(
         CacheContentImpl(0L, "A"),
         cache.cache.value
      )
   }

   private external fun `deserialize$preparePanoptiqon`()
   private external fun `deserialize$loadCache`(cacheId: CacheId): Cache<*>

   @Test
   fun deserialize_writable() {
      `deserialize_writable$preparePanoptiqon`()

      val json = createJsonSerialization(::`deserialize_writable$loadCache`)

      val cacheContainerJson = cacheContainerJsonStr(
         "CacheSerializerTest/deserialize_writable",
         "CacheSerializerTest/deserialize_writable/0"
      )
      val cache = json.decodeFromString<WritableCacheContainer>(cacheContainerJson)

      assertEquals(
         CacheContentImpl(0L, "A"),
         cache.cache.value
      )
   }

   private external fun `deserialize_writable$preparePanoptiqon`()
   private external fun `deserialize_writable$loadCache`(cacheId: CacheId): WritableCache<*>

   @Test
   fun deserialize_cacheNotFound() {
      `deserialize_cacheNotFound$preparePanoptiqon`()

      val json = createJsonSerialization(::`deserialize_cacheNotFound$loadCache`)

      val cacheContainerJson = cacheContainerJsonStr(
         "CacheSerializerTest/deserialize_cacheNotFound",
         "CacheSerializerTest/deserialize_cacheNotFound/1"
      )

      assertFailsWith<IOException> {
         json.decodeFromString<CacheContainer>(cacheContainerJson)
      }
   }

   private external fun `deserialize_cacheNotFound$preparePanoptiqon`()
   private external fun `deserialize_cacheNotFound$loadCache`(cacheId: CacheId): Cache<*>

   @Test
   fun deserialize_writable_cacheNotFound() {
      `deserialize_writable_cacheNotFound$preparePanoptiqon`()

      val json = createJsonSerialization(::`deserialize_writable_cacheNotFound$loadCache`)

      val cacheContainerJson = cacheContainerJsonStr(
         "CacheSerializerTest/deserialize_writable_cacheNotFound",
         "CacheSerializerTest/deserialize_writable_cacheNotFound/1"
      )

      assertFailsWith<IOException> {
         json.decodeFromString<WritableCacheContainer>(cacheContainerJson)
      }
   }

   private external fun `deserialize_writable_cacheNotFound$preparePanoptiqon`()
   private external fun `deserialize_writable_cacheNotFound$loadCache`(cacheId: CacheId): WritableCache<*>
}
