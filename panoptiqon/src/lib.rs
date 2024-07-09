/*
 * Copyright 2024 wcaokaze
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

use std::ops::{Deref, DerefMut};

pub struct Cache<T> {
   value: T
}

impl<T> Cache<T> {
   fn new(value: T) -> Self {
      Cache {
         value
      }
   }
}

impl<T> Deref for Cache<T> {
   type Target = T;

   fn deref(&self) -> &T {
      &self.value
   }
}

impl<T> DerefMut for Cache<T> {
   fn deref_mut(&mut self) -> &mut T {
      &mut self.value
   }
}

#[cfg(test)]
mod tests {
   use super::Cache;

   #[test]
   fn deref() {
      let mut cache = Cache::new(42);
      assert_eq!(42, cache.value);

      assert_eq!(42, *cache);

      *cache = 43;
      assert_eq!(43, cache.value);
   }
}
