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

package com.wcaokaze.probosqis.cache.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.Saver
import com.wcaokaze.probosqis.cache.core.WritableCache
import kotlinx.serialization.json.JsonObject

@Composable
fun <T, Saveable : Any> rememberIn(
   cache: WritableCache<JsonObject>,
   key: String,
   saver: Saver<T, Saveable>,
   init: () -> T
): T {
   // TODO
   return remember { init() }
}
