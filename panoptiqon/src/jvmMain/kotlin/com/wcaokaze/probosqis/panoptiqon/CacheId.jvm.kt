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

import java.nio.charset.Charset

actual class CacheId
   internal constructor(
      val repositoryDirPath: ByteArray,
      val filePath: ByteArray
   )
{
   override fun hashCode(): Int {
      var h = 1
      h = h * 31 + repositoryDirPath.contentHashCode()
      h = h * 31 + filePath         .contentHashCode()
      return h
   }

   override fun equals(other: Any?): Boolean {
      return other is CacheId
          && repositoryDirPath.contentEquals(other.repositoryDirPath)
          && filePath         .contentEquals(other.filePath)
   }

   override fun toString() = buildString {
      append("CacheId(")
      append("repo=")
      append(String(repositoryDirPath, Charset.defaultCharset()))
      append(",")
      append("file=")
      append(String(filePath, Charset.defaultCharset()))
      append(")")
   }
}
