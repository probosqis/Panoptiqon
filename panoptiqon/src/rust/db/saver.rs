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
