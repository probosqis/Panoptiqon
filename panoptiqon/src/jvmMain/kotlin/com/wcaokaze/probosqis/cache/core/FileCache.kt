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

import androidx.compose.runtime.mutableStateOf
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.KSerializer
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.decodeFromStream
import kotlinx.serialization.json.encodeToStream
import kotlinx.serialization.serializer
import java.io.File
import java.io.IOException

@RequiresOptIn
annotation class TemporaryCacheApi

@TemporaryCacheApi
inline fun <reified T> saveCache(
   value: T,
   file: File,
   json: Json
): WritableCache<T> {
   return saveCache(value, file, json, serializer())
}

@TemporaryCacheApi
fun <T> saveCache(
   value: T,
   file: File,
   json: Json,
   serializer: KSerializer<T>
): WritableCache<T> {
   return FileCache(value, file, json, serializer)
      .apply { saveCache(value) }
}

/**
 * 取り急ぎのAPIのため、同じFileを複数回loadCacheしたときの返り値は同一ではない。
 * 同様に[saveCache]したときの返り値とも同一ではない。
 *
 * @throws IOException
 */
@TemporaryCacheApi
inline fun <reified T> loadCache(file: File, json: Json): WritableCache<T> {
   return loadCache(file, json, serializer())
}

@TemporaryCacheApi
inline fun <reified T> loadCacheOrDefault(
   file: File,
   json: Json,
   default: () -> T
): WritableCache<T> {
   return try {
      loadCache(file, json)
   } catch (e: IOException) {
      val defaultValue = default()
      saveCache(defaultValue, file, json)
   }
}

@TemporaryCacheApi
fun <T> loadCache(
   file: File,
   json: Json,
   serializer: KSerializer<T>
): WritableCache<T> {
   val value = file.inputStream().buffered().use {
      @OptIn(ExperimentalSerializationApi::class)
      json.decodeFromStream(serializer, it)
   }

   return FileCache(value, file, json, serializer)
}

private class FileCache<T>(
   initialValue: T,
   private val file: File,
   private val json: Json,
   private val serializer: KSerializer<T>
) : Cache<T>, WritableCache<T> {
   private val _state = mutableStateOf(initialValue)

   @InternalCacheApi
   override val state get() = _state

   @InternalCacheApi
   override val mutableState get() = _state

   override var value: T
      get() = _state.value
      set(value) {
         _state.value = value
         saveCache(value)
      }

   override fun asCache(): Cache<T> = this

   fun saveCache(value: T) {
      val dir = file.absoluteFile.parentFile
      if (!dir.exists()) {
         dir.mkdirs()
      }

      file.outputStream().buffered().use {
         @OptIn(ExperimentalSerializationApi::class)
         json.encodeToStream(serializer, value, it)
      }
   }
}
