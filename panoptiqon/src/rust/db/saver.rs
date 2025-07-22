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

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use crate::db::save_task::SaveTask;

pub(crate) type DirPath = Arc<PathBuf>;
pub(crate) type Serializer<'a> = &'a mut serde_json::Serializer<BufWriter<File>>;

pub(crate) struct Saver {
   #[cfg(any(test, feature = "testable"))]
   received_tasks: Vec<SaveTask>
}

impl Saver {
   pub(crate) const fn new() -> Self {
      Self {
         #[cfg(any(test, feature = "testable"))]
         received_tasks: Vec::new()
      }
   }

   #[cfg(not(any(test, feature = "testable")))]
   pub(crate) fn save(&mut self, task: SaveTask) -> anyhow::Result<()> {
      self._save(task)
   }

   fn _save(&mut self, task: SaveTask) -> anyhow::Result<()> {
      use std::fs;

      let file_path = task.file_path();

      if let Some(parent) = file_path.parent() {
         fs::create_dir_all(parent)?;
      }

      let file = File::options()
         .create(true)
         .write(true)
         .truncate(true)
         .open(file_path)?;

      let writer = BufWriter::new(file);
      let mut serializer = serde_json::Serializer::new(writer);

      task.cache.serialize(&mut serializer)?;

      Ok(())
   }

   #[cfg(any(test, feature = "testable"))]
   pub(crate) fn save(&mut self, task: SaveTask) -> anyhow::Result<()> {
      self.received_tasks.push(task);
      Ok(())
   }

   #[cfg(any(test, feature = "testable"))]
   pub(crate) fn clear_received_tasks(&mut self) -> Vec<SaveTask> {
      use std::mem;
      mem::replace(&mut self.received_tasks, Vec::new())
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::Path;

   #[test]
   fn save() {
      use std::fs;
      use std::path::PathBuf;
      use std::sync::Arc;
      use scopeguard::defer;
      use serde::Serialize;
      use crate::db::savable::Savable;
      use crate::db::save_task::SaveTask;
      use super::{Saver, Serializer};

      #[derive(Serialize)]
      struct SavableImpl {
         id: i32,
         value: String
      }

      impl Savable for SavableImpl {
         fn repository_dir_path(&self) -> &Path {
            Path::new("test/Saver/save")
         }

         fn file_path(&self) -> PathBuf {
            use std::path::Path;
            Path::new("test/Saver/save").join(self.id.to_string())
         }

         fn serialize(
            &self,
            serializer: Serializer
         ) -> anyhow::Result<
            <Serializer as serde::Serializer>::Ok,
            <Serializer as serde::Serializer>::Error
         > {
            Serialize::serialize(self, serializer)
         }
      }

      let savable = SavableImpl {
         id: 0,
         value: "value".to_string()
      };

      let mut saver = Saver::new();
      let task = SaveTask::new(Arc::new(savable));
      let result = saver._save(task);
      assert!(result.is_ok());

      defer! {
         fs::remove_file("test/Saver/save/0").unwrap()
      }

      let saved_text = fs::read_to_string("test/Saver/save/0").unwrap();

      assert_eq!(
         r#"{"id":0,"value":"value"}"#,
         &saved_text
      );
   }

   #[test]
   fn save_mkdir() {
      use std::fs;
      use std::path::PathBuf;
      use std::sync::Arc;
      use scopeguard::defer;
      use serde::Serialize;
      use crate::db::savable::Savable;
      use crate::db::save_task::SaveTask;
      use super::{Saver, Serializer};

      #[derive(Serialize)]
      struct SavableImpl {
         id: i32,
         value: String
      }

      impl Savable for SavableImpl {
         fn repository_dir_path(&self) -> &Path {
            Path::new("test/Saver/save_mkdir")
         }

         fn file_path(&self) -> PathBuf {
            use std::path::Path;
            Path::new("test/Saver/save_mkdir").join(self.id.to_string())
         }

         fn serialize(
            &self,
            serializer: Serializer
         ) -> anyhow::Result<
            <Serializer as serde::Serializer>::Ok,
            <Serializer as serde::Serializer>::Error
         > {
            Serialize::serialize(self, serializer)
         }
      }

      if fs::exists("test/Saver/save_mkdir").unwrap() {
         fs::remove_dir_all("test/Saver/save_mkdir").unwrap();
      }

      let savable = SavableImpl {
         id: 0,
         value: "value".to_string()
      };

      let mut saver = Saver::new();
      let task = SaveTask::new(Arc::new(savable));
      saver._save(task).unwrap();

      defer! {
         fs::remove_dir_all("test/Saver/save_mkdir").unwrap();
      }

      assert!(fs::exists("test/Saver/save_mkdir").unwrap());
   }
}
