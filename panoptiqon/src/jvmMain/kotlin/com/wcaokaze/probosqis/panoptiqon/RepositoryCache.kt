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

import androidx.compose.runtime.State
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationException
import kotlinx.serialization.builtins.NothingSerializer
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder

@Serializable(with = RepositoryCacheSerializer::class)
actual class RepositoryCache<T>
   internal constructor(
      private val uniqueCache: UniqueCache<T>
   )
   : Cache<T>
{
   val uniqueCacheRustStateAddress: Long
      get() = uniqueCache.nativeStateAddress

   @InternalCacheApi
   override val state: State<T> get() = uniqueCache.state

   override val id: Cache.Id
      get() = uniqueCache.id

   override val value: T by uniqueCache::value

   override fun hashCode() = uniqueCache.hashCode()
   override fun equals(other: Any?)
       = other is RepositoryCache<*> && uniqueCache === other.uniqueCache
}

@Serializable(with = WritableRepositoryCacheSerializer::class)
actual class WritableRepositoryCache<T>
   internal constructor(
      private val uniqueCache: WritableUniqueCache<T>
   )
   : Cache<T>, WritableCache<T>
{
   val uniqueCacheRustStateAddress: Long
      get() = uniqueCache.nativeStateAddress

   @InternalCacheApi
   override val state: State<T> get() = uniqueCache.state

   @InternalCacheApi
   override val mutableState get() = uniqueCache.state

   override val id: Cache.Id
      get() = uniqueCache.id

   override var value: T by uniqueCache::value

   override fun asCache(): Cache<T> = this

   override fun hashCode() = uniqueCache.hashCode()
   override fun equals(other: Any?)
       = other is WritableRepositoryCache<*> && uniqueCache === other.uniqueCache
}

actual object RepositoryCacheSerializer : KSerializer<RepositoryCache<*>> {
   override val descriptor = SerialDescriptor(
      "RepositoryCache",
      @OptIn(ExperimentalSerializationApi::class)
      NothingSerializer().descriptor
   )

   override fun serialize(encoder: Encoder, value: RepositoryCache<*>) {
   }

   override fun deserialize(decoder: Decoder): RepositoryCache<*> {
      throw SerializationException()
   }
}

actual object WritableRepositoryCacheSerializer : KSerializer<WritableRepositoryCache<*>> {
   override val descriptor = SerialDescriptor(
      "WritableRepositoryCache",
      @OptIn(ExperimentalSerializationApi::class)
      NothingSerializer().descriptor
   )

   override fun serialize(encoder: Encoder, value: WritableRepositoryCache<*>) {
   }

   override fun deserialize(decoder: Decoder): WritableRepositoryCache<*> {
      throw SerializationException()
   }
}
