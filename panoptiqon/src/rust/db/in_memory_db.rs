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

#![cfg(any(test, feature = "testable"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct InMemoryDb {
   files: HashMap<PathBuf, Vec<u8>>
}

impl InMemoryDb {
   pub fn new() -> InMemoryDb {
      InMemoryDb {
         files: HashMap::new()
      }
   }

   pub(crate) fn read(&self, file_path: impl AsRef<Path>) -> anyhow::Result<&Vec<u8>> {
      use anyhow::Context;

      self.files.get(file_path.as_ref()).context("File not found")
   }

   pub(crate) fn write(&mut self, file_path: impl AsRef<Path>) -> &mut Vec<u8> {
      let file_path = file_path.as_ref().to_path_buf();

      if !self.files.contains_key(&file_path) {
         self.files.insert(PathBuf::clone(&file_path), Vec::new());
      }

      self.files.get_mut(&file_path).unwrap()
   }

   pub(crate) fn clear(&mut self) {
      self.files.clear();
   }
}
