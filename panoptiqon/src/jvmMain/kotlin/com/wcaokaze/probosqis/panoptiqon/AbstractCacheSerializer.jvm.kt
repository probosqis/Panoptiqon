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

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.ByteArraySerializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.buildClassSerialDescriptor
import kotlinx.serialization.encoding.CompositeDecoder
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.encoding.decodeStructure

private class Impl(serialName: String) {
   private val byteArraySerializer = ByteArraySerializer()

   val descriptor = buildClassSerialDescriptor(serialName) {
      element("repositoryDirPath", byteArraySerializer.descriptor)
      element("filePath",          byteArraySerializer.descriptor)
   }

   fun serializeCacheId(encoder: Encoder, cacheId: CacheId) {
      val structureEncoder = encoder.beginStructure(descriptor)
      structureEncoder.encodeSerializableElement(descriptor, 0, byteArraySerializer, cacheId.repositoryDirPath)
      structureEncoder.encodeSerializableElement(descriptor, 1, byteArraySerializer, cacheId.filePath)
      structureEncoder.endStructure(descriptor)
   }

   fun deserializeCacheId(decoder: Decoder): CacheId {
      return decoder.decodeStructure(descriptor) {
         @OptIn(ExperimentalSerializationApi::class)
         if (decodeSequentially()) {
            val repositoryDirPath = decodeSerializableElement(descriptor, 0, byteArraySerializer)
            val filePath          = decodeSerializableElement(descriptor, 1, byteArraySerializer)
            CacheId(repositoryDirPath, filePath)
         } else {
            var repositoryDirPath: ByteArray? = null
            var filePath:          ByteArray? = null

            loop@while (true) {
               when (val index = decodeElementIndex(descriptor)) {
                  CompositeDecoder.DECODE_DONE -> break@loop
                  0 -> { repositoryDirPath = decodeSerializableElement(descriptor, 0, byteArraySerializer) }
                  1 -> { filePath          = decodeSerializableElement(descriptor, 1, byteArraySerializer) }
                  else -> throw SerializationException("Invalid index: $index")
               }
            }

            if (repositoryDirPath == null) { throw SerializationException("repositoryDirPath is missing") }
            if (filePath          == null) { throw SerializationException("repositoryDirPath is missing") }

            CacheId(repositoryDirPath, filePath)
         }
      }
   }
}

actual abstract class AbstractCacheSerializer<T> : KSerializer<Cache<T>> {
   private val impl = Impl("com.wcaokaze.probosqis.panoptiqon.Cache")

   protected actual abstract fun loadCache(cacheId: CacheId): Cache<T>

   actual final override val descriptor: SerialDescriptor
      get() = impl.descriptor

   actual final override fun serialize(encoder: Encoder, value: Cache<T>) {
      val cacheId = value.id
      impl.serializeCacheId(encoder, cacheId)
   }

   actual final override fun deserialize(decoder: Decoder): Cache<T> {
      val cacheId = impl.deserializeCacheId(decoder)
      return loadCache(cacheId)
   }
}

actual abstract class AbstractWritableCacheSerializer<T> : KSerializer<WritableCache<T>> {
   private val impl = Impl("com.wcaokaze.probosqis.panoptiqon.WritableCache")

   protected actual abstract fun loadCache(cacheId: CacheId): WritableCache<T>

   actual final override val descriptor: SerialDescriptor
      get() = impl.descriptor

   actual final override fun serialize(encoder: Encoder, value: WritableCache<T>) {
      val cacheId = value.id
      impl.serializeCacheId(encoder, cacheId)
   }

   actual final override fun deserialize(decoder: Decoder): WritableCache<T> {
      val cacheId = impl.deserializeCacheId(decoder)
      return loadCache(cacheId)
   }
}
