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

pub(crate) struct Saver;

impl Saver {
   pub(crate) fn save(task: SaveTask) -> anyhow::Result<()> {
      let file = File::options()
         .create(true)
         .write(true)
         .truncate(true)
         .open(task.file_path())?;

      let writer = BufWriter::new(file);
      let mut serializer = serde_json::Serializer::new(writer);

      task.cache.serialize(&mut serializer)?;

      Ok(())
   }
}

#[cfg(test)]
mod test {
   use super::{DirPath, Saver, Serializer};

   #[test]
   fn save() {
      use std::fs;
      use std::path::{Path, PathBuf};
      use std::sync::Arc;
      use scopeguard::defer;
      use serde::Serialize;
      use crate::db::savable::Savable;
      use crate::db::save_task::SaveTask;

      #[derive(Serialize)]
      struct SavableImpl {
         id: i32,
         value: String
      }

      impl Savable for SavableImpl {
         fn file_path(&self, dir_path: &Path) -> PathBuf {
            dir_path.join(self.id.to_string())
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

      let dir_path = DirPath::new(PathBuf::from("test/Saver/save"));
      let task = SaveTask::new(&dir_path, Arc::new(savable));
      let result = Saver::save(task);
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
}
